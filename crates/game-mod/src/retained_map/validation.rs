// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::binding::{
    RetainedMapLiveBinding, RetainedMapNodeVisibility, validate_identity, validate_text,
};
use super::error::RetainedMapError;
use super::measure::{node_bytes, snapshot_bytes};
use super::model::{
    RETAINED_MAP_MAX_DETAIL_BYTES, RETAINED_MAP_MAX_EDGES, RETAINED_MAP_MAX_NODES,
    RETAINED_MAP_MAX_SNAPSHOT_BYTES, RETAINED_MAP_MAX_TRAVEL_BINDINGS,
    RETAINED_MAP_PRODUCER_VERSION, RetainedMapContents, RetainedMapEdge, RetainedMapNode,
    RetainedMapNodeInput, RetainedMapNodeKind, RetainedMapSnapshot, RetainedMapSnapshotInput,
    RetainedMapTravel,
};

pub(super) fn validate_snapshot(
    input: RetainedMapSnapshotInput,
) -> Result<RetainedMapSnapshot, RetainedMapError> {
    validate_binding(&input.binding)?;
    if input.nodes.len() > RETAINED_MAP_MAX_NODES {
        return Err(RetainedMapError::InvalidInput("nodes"));
    }
    if input.edges.len() > RETAINED_MAP_MAX_EDGES {
        return Err(RetainedMapError::InvalidInput("edges"));
    }
    if input.travel.len() > RETAINED_MAP_MAX_TRAVEL_BINDINGS {
        return Err(RetainedMapError::InvalidInput("travel"));
    }

    let mut nodes = BTreeMap::new();
    for node in &input.nodes {
        validate_node(node)?;
        let detail = node_bytes(node);
        if detail > RETAINED_MAP_MAX_DETAIL_BYTES {
            return Err(RetainedMapError::DetailTooLarge {
                limit: RETAINED_MAP_MAX_DETAIL_BYTES,
                actual: detail,
            });
        }
        let retained = retained_node(node);
        if nodes.insert(node.node_id.clone(), retained).is_some() {
            return Err(RetainedMapError::DuplicateNode(node.node_id.clone()));
        }
    }
    let total = snapshot_bytes(&input);
    if total > RETAINED_MAP_MAX_SNAPSHOT_BYTES {
        return Err(RetainedMapError::DetailTooLarge {
            limit: RETAINED_MAP_MAX_SNAPSHOT_BYTES,
            actual: total,
        });
    }

    let edges = validate_edges(&input.edges, &nodes)?;
    let travel = validate_travel(&input.travel, &nodes)?;
    Ok(RetainedMapSnapshot {
        binding: input.binding,
        provenance: input.provenance,
        nodes,
        edges,
        travel,
    })
}

pub(super) fn validate_binding(binding: &RetainedMapLiveBinding) -> Result<(), RetainedMapError> {
    if binding.catalog.producer_version != RETAINED_MAP_PRODUCER_VERSION {
        return Err(RetainedMapError::InvalidBinding("producer_version"));
    }
    for (field, value) in [
        (
            "content_manifest.adapter_compatibility",
            binding
                .catalog
                .content_manifest
                .adapter_compatibility
                .as_str(),
        ),
        (
            "content_manifest.content_set_revision",
            binding
                .catalog
                .content_manifest
                .content_set_revision
                .as_str(),
        ),
        (
            "content_manifest.localized_text_revision",
            binding
                .catalog
                .content_manifest
                .localized_text_revision
                .as_str(),
        ),
        (
            "content_manifest.inventory_revision",
            binding.catalog.content_manifest.inventory_revision.as_str(),
        ),
        ("locale", binding.catalog.locale.as_str()),
        ("game_instance_id", binding.game_instance_id.as_str()),
        ("run_id", binding.run_id.as_str()),
        ("act_id", binding.act_id.as_str()),
        ("mode_id", binding.mode_id.as_str()),
        ("map_instance_id", binding.map_instance_id.as_str()),
        ("snapshot_id", binding.snapshot_id.as_str()),
    ] {
        validate_identity(value, field)?;
    }
    Ok(())
}

fn validate_node(node: &RetainedMapNodeInput) -> Result<(), RetainedMapError> {
    validate_identity(&node.node_id, "node_id")?;
    if let RetainedMapNodeKind::Unsupported(kind) = &node.kind {
        validate_text(kind, "node_kind")?;
    }
    if let Some(label) = node.label.value() {
        validate_text(label, "node_label")?;
    }
    let hidden = !matches!(
        node.visibility,
        RetainedMapNodeVisibility::Public | RetainedMapNodeVisibility::OwnerOnly
    );
    if hidden
        && (node.label.value().is_some()
            || matches!(node.contents, RetainedMapContents::PublicCategory))
    {
        return Err(RetainedMapError::InvalidInput("hidden node detail"));
    }
    Ok(())
}

fn retained_node(node: &RetainedMapNodeInput) -> RetainedMapNode {
    RetainedMapNode {
        node_id: node.node_id.clone(),
        kind: node.kind.clone(),
        visibility: node.visibility,
        label: node.label.clone(),
        contents: node.contents,
    }
}

fn validate_edges(
    inputs: &[super::model::RetainedMapEdgeInput],
    nodes: &BTreeMap<String, RetainedMapNode>,
) -> Result<Vec<RetainedMapEdge>, RetainedMapError> {
    let mut seen = BTreeSet::new();
    let mut edges = Vec::with_capacity(inputs.len());
    for edge in inputs {
        validate_identity(&edge.from_node_id, "from_node_id")?;
        validate_identity(&edge.to_node_id, "to_node_id")?;
        if edge.from_node_id == edge.to_node_id {
            return Err(RetainedMapError::InvalidInput("self_edge"));
        }
        for endpoint in [&edge.from_node_id, &edge.to_node_id] {
            if !nodes.contains_key(endpoint) {
                return Err(RetainedMapError::UnknownNode(endpoint.clone()));
            }
        }
        if !seen.insert((edge.from_node_id.clone(), edge.to_node_id.clone())) {
            return Err(RetainedMapError::DuplicateEdge {
                from: edge.from_node_id.clone(),
                to: edge.to_node_id.clone(),
            });
        }
        edges.push(RetainedMapEdge {
            from_node_id: edge.from_node_id.clone(),
            to_node_id: edge.to_node_id.clone(),
        });
    }
    Ok(edges)
}

fn validate_travel(
    inputs: &[super::model::RetainedMapTravelInput],
    nodes: &BTreeMap<String, RetainedMapNode>,
) -> Result<BTreeMap<String, RetainedMapTravel>, RetainedMapError> {
    let mut travel = BTreeMap::new();
    for entry in inputs {
        validate_identity(&entry.action_id, "action_id")?;
        validate_identity(&entry.from_node_id, "from_node_id")?;
        validate_identity(&entry.to_node_id, "to_node_id")?;
        if entry.action_id == entry.from_node_id || entry.action_id == entry.to_node_id {
            return Err(RetainedMapError::AmbiguousIdentity("action_id"));
        }
        for endpoint in [&entry.from_node_id, &entry.to_node_id] {
            if !nodes.contains_key(endpoint) {
                return Err(RetainedMapError::UnknownNode(endpoint.clone()));
            }
        }
        let retained = RetainedMapTravel {
            from_node_id: entry.from_node_id.clone(),
            to_node_id: entry.to_node_id.clone(),
            action_id: entry.action_id.clone(),
        };
        if travel.insert(entry.action_id.clone(), retained).is_some() {
            return Err(RetainedMapError::DuplicateTravel(entry.action_id.clone()));
        }
    }
    Ok(travel)
}
