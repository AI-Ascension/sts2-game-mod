// SPDX-License-Identifier: MIT

use super::binding::RetainedMapLiveBinding;
use super::field::RetainedMapField;
use super::model::{
    RetainedMapEdgeInput, RetainedMapNodeInput, RetainedMapNodeKind, RetainedMapSnapshotInput,
    RetainedMapTravelInput,
};

const NODE_OVERHEAD_BYTES: usize = 64;
const EDGE_OVERHEAD_BYTES: usize = 32;
const TRAVEL_OVERHEAD_BYTES: usize = 64;

pub(super) fn snapshot_bytes(input: &RetainedMapSnapshotInput) -> usize {
    let mut bytes = 0;
    add_binding_bytes(&mut bytes, &input.binding);
    for node in &input.nodes {
        bytes = bytes.saturating_add(node_bytes(node));
    }
    for edge in &input.edges {
        bytes = bytes.saturating_add(edge_bytes(edge));
    }
    for travel in &input.travel {
        bytes = bytes.saturating_add(travel_bytes(travel));
    }
    bytes
}

pub(super) fn node_bytes(node: &RetainedMapNodeInput) -> usize {
    let mut bytes = NODE_OVERHEAD_BYTES;
    bytes = bytes.saturating_add(node.node_id.len());
    if let RetainedMapNodeKind::Unsupported(kind) = &node.kind {
        bytes = bytes.saturating_add(kind.len());
    }
    add_field_text(&mut bytes, &node.label);
    bytes
}

fn edge_bytes(edge: &RetainedMapEdgeInput) -> usize {
    EDGE_OVERHEAD_BYTES
        .saturating_add(edge.from_node_id.len())
        .saturating_add(edge.to_node_id.len())
}

fn travel_bytes(travel: &RetainedMapTravelInput) -> usize {
    TRAVEL_OVERHEAD_BYTES
        .saturating_add(travel.from_node_id.len())
        .saturating_add(travel.to_node_id.len())
        .saturating_add(travel.action_id.len())
}

fn add_binding_bytes(bytes: &mut usize, binding: &RetainedMapLiveBinding) {
    for value in [
        binding
            .catalog
            .content_manifest
            .adapter_compatibility
            .as_str(),
        binding
            .catalog
            .content_manifest
            .content_set_revision
            .as_str(),
        binding
            .catalog
            .content_manifest
            .localized_text_revision
            .as_str(),
        binding.catalog.content_manifest.inventory_revision.as_str(),
        binding.catalog.locale.as_str(),
        binding.catalog.producer_version.as_str(),
        binding.game_instance_id.as_str(),
        binding.run_id.as_str(),
        binding.act_id.as_str(),
        binding.mode_id.as_str(),
        binding.map_instance_id.as_str(),
        binding.snapshot_id.as_str(),
    ] {
        *bytes = bytes.saturating_add(value.len());
    }
}

fn add_field_text(bytes: &mut usize, field: &RetainedMapField<String>) {
    if let Some(value) = field.value() {
        *bytes = bytes.saturating_add(value.len());
    }
}
