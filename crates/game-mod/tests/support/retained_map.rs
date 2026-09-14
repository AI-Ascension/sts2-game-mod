// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCursorBinding, RETAINED_MAP_PRODUCER_VERSION, RetainedMapCatalogBinding,
    RetainedMapContents, RetainedMapEdgeInput, RetainedMapField, RetainedMapLiveBinding,
    RetainedMapNodeInput, RetainedMapNodeKind, RetainedMapNodeVisibility, RetainedMapProvenance,
    RetainedMapSnapshotInput, RetainedMapTravelInput, RetainedMapUnavailableReason,
};

pub fn content_manifest() -> ContentCursorBinding {
    ContentCursorBinding {
        catalog_generation: 7,
        adapter_compatibility: "adapter:fixture".to_owned(),
        content_set_revision: "content:fixture".to_owned(),
        localized_text_revision: "text:fixture:en-US".to_owned(),
        inventory_revision: "inventory:fixture".to_owned(),
    }
}

pub fn catalog() -> RetainedMapCatalogBinding {
    RetainedMapCatalogBinding {
        content_manifest: content_manifest(),
        locale: "en-US".to_owned(),
        producer_version: RETAINED_MAP_PRODUCER_VERSION.to_owned(),
    }
}

pub fn binding(epoch: u64) -> RetainedMapLiveBinding {
    RetainedMapLiveBinding {
        catalog: catalog(),
        game_instance_id: "game:fixture".to_owned(),
        run_id: "run:fixture".to_owned(),
        act_id: "act:1".to_owned(),
        mode_id: "mode:standard".to_owned(),
        map_instance_id: "map-instance:1".to_owned(),
        snapshot_id: format!("snapshot:{epoch}"),
        epoch,
    }
}

pub fn label(text: &str) -> RetainedMapField<String> {
    RetainedMapField::Available(text.to_owned())
}

pub fn node(
    node_id: &str,
    kind: RetainedMapNodeKind,
    visibility: RetainedMapNodeVisibility,
) -> RetainedMapNodeInput {
    RetainedMapNodeInput {
        node_id: node_id.to_owned(),
        kind,
        visibility,
        label: label(&format!("Label {node_id}")),
        contents: RetainedMapContents::PublicCategory,
    }
}

pub fn hidden_node(node_id: &str) -> RetainedMapNodeInput {
    RetainedMapNodeInput {
        node_id: node_id.to_owned(),
        kind: RetainedMapNodeKind::Event,
        visibility: RetainedMapNodeVisibility::Hidden,
        label: RetainedMapField::Hidden,
        contents: RetainedMapContents::Withheld(RetainedMapUnavailableReason::PolicyWithheld),
    }
}

pub fn edge(from: &str, to: &str) -> RetainedMapEdgeInput {
    RetainedMapEdgeInput {
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
    }
}

pub fn travel(from: &str, to: &str, action_id: &str) -> RetainedMapTravelInput {
    RetainedMapTravelInput {
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
        action_id: action_id.to_owned(),
    }
}

pub fn nodes() -> Vec<RetainedMapNodeInput> {
    vec![
        node(
            "map:1:0:0",
            RetainedMapNodeKind::Start,
            RetainedMapNodeVisibility::Public,
        ),
        node(
            "map:1:1:0",
            RetainedMapNodeKind::Combat,
            RetainedMapNodeVisibility::Public,
        ),
        node(
            "map:1:1:1",
            RetainedMapNodeKind::Event,
            RetainedMapNodeVisibility::Public,
        ),
        node(
            "map:1:2:0",
            RetainedMapNodeKind::Boss,
            RetainedMapNodeVisibility::Public,
        ),
    ]
}

pub fn edges() -> Vec<RetainedMapEdgeInput> {
    vec![
        edge("map:1:0:0", "map:1:1:0"),
        edge("map:1:0:0", "map:1:1:1"),
        edge("map:1:1:0", "map:1:2:0"),
        edge("map:1:1:1", "map:1:2:0"),
    ]
}

pub fn travel_bindings() -> Vec<RetainedMapTravelInput> {
    vec![
        travel("map:1:0:0", "map:1:1:0", "select-map-node:1:left"),
        travel("map:1:0:0", "map:1:1:1", "select-map-node:1:right"),
    ]
}

pub fn snapshot(epoch: u64) -> RetainedMapSnapshotInput {
    RetainedMapSnapshotInput {
        binding: binding(epoch),
        provenance: RetainedMapProvenance::ObservedPublic,
        nodes: nodes(),
        edges: edges(),
        travel: travel_bindings(),
    }
}

pub fn long_label_snapshot(epoch: u64, label_len: usize) -> RetainedMapSnapshotInput {
    let long = "L".repeat(label_len);
    let mut input = snapshot(epoch);
    for node in &mut input.nodes {
        node.label = label(&long);
    }
    input
}

pub fn page_snapshot(epoch: u64, count: usize) -> RetainedMapSnapshotInput {
    let nodes = (0..count)
        .map(|index| {
            node(
                &format!("map:1:0:{index}"),
                RetainedMapNodeKind::Combat,
                RetainedMapNodeVisibility::Public,
            )
        })
        .collect();
    RetainedMapSnapshotInput {
        binding: binding(epoch),
        provenance: RetainedMapProvenance::ObservedPublic,
        nodes,
        edges: Vec::new(),
        travel: Vec::new(),
    }
}

pub fn oversized_node_snapshot(
    epoch: u64,
    kind_len: usize,
    label_len: usize,
) -> RetainedMapSnapshotInput {
    RetainedMapSnapshotInput {
        binding: binding(epoch),
        provenance: RetainedMapProvenance::ObservedPublic,
        nodes: vec![RetainedMapNodeInput {
            node_id: "map:1:0:0".to_owned(),
            kind: RetainedMapNodeKind::Unsupported("k".repeat(kind_len)),
            visibility: RetainedMapNodeVisibility::Public,
            label: label(&"L".repeat(label_len)),
            contents: RetainedMapContents::Unknown,
        }],
        edges: Vec::new(),
        travel: Vec::new(),
    }
}

pub fn bulk_snapshot(epoch: u64, count: usize, label_len: usize) -> RetainedMapSnapshotInput {
    let long = "L".repeat(label_len);
    let nodes = (0..count)
        .map(|index| RetainedMapNodeInput {
            node_id: format!("map:1:0:{index}"),
            kind: RetainedMapNodeKind::Combat,
            visibility: RetainedMapNodeVisibility::Public,
            label: label(&long),
            contents: RetainedMapContents::PublicCategory,
        })
        .collect();
    RetainedMapSnapshotInput {
        binding: binding(epoch),
        provenance: RetainedMapProvenance::ObservedPublic,
        nodes,
        edges: Vec::new(),
        travel: Vec::new(),
    }
}
