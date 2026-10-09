// SPDX-License-Identifier: MIT

use crate::fixture::{binding, catalog, duration, instance, integer, source};
use sts2_game_mod::{
    PowerStatusField, PowerStatusLiveReader, PowerStatusLiveSnapshot, PowerStatusLiveSnapshotInput,
    PowerStatusOwnerKind, PowerStatusSourceKind,
};

#[test]
fn same_definition_and_owner_keep_distinct_source_instances() {
    let catalog = catalog();
    let mut card = instance(
        "instance:strength:card-source",
        "mod:synthetic:strength",
        PowerStatusOwnerKind::Player,
        PowerStatusField::Available(integer(2, "count")),
        PowerStatusField::Available(duration(2, "turn")),
    );
    card.source = PowerStatusField::Available(source(
        PowerStatusSourceKind::Card,
        "card:strength-application",
    ));
    let mut relic = instance(
        "instance:strength:relic-source",
        "mod:synthetic:strength",
        PowerStatusOwnerKind::Player,
        PowerStatusField::Available(integer(4, "count")),
        PowerStatusField::Available(duration(2, "turn")),
    );
    relic.source = PowerStatusField::Available(source(
        PowerStatusSourceKind::Relic,
        "relic:strength-application",
    ));
    assert_eq!(card.definition_id, relic.definition_id);
    assert_eq!(card.owner, relic.owner);
    assert_ne!(card.instance_id, relic.instance_id);

    let snapshot = PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
        binding: binding(&catalog, 7),
        instances: vec![card, relic],
    })
    .expect("same-owner source instances");
    let reader = PowerStatusLiveReader::new(&catalog, snapshot).expect("live reader");
    let references = reader.references();
    assert_eq!(references.len(), 2);
    let card_reference = references
        .iter()
        .find(|reference| reference.instance_id == "instance:strength:card-source")
        .expect("card source reference");
    let relic_reference = references
        .iter()
        .find(|reference| reference.instance_id == "instance:strength:relic-source")
        .expect("relic source reference");
    assert_eq!(card_reference.definition_id, relic_reference.definition_id);
    assert_eq!(card_reference.owner_kind, relic_reference.owner_kind);
    assert_eq!(card_reference.owner_id, relic_reference.owner_id);

    let card_detail = reader.get(card_reference).expect("card source detail");
    let relic_detail = reader.get(relic_reference).expect("relic source detail");
    assert_eq!(card_detail.owner, relic_detail.owner);
    assert_eq!(card_detail.owner.id.as_str(), "entity:1");
    assert_eq!(
        card_detail.source.value().expect("card source").kind,
        PowerStatusSourceKind::Card
    );
    assert_eq!(
        card_detail.source.value().expect("card source").id.as_str(),
        "card:strength-application"
    );
    assert_eq!(
        relic_detail.source.value().expect("relic source").kind,
        PowerStatusSourceKind::Relic
    );
    assert_eq!(
        relic_detail
            .source
            .value()
            .expect("relic source")
            .id
            .as_str(),
        "relic:strength-application"
    );
    assert_eq!(
        card_detail.amount.value().expect("card amount"),
        &integer(2, "count")
    );
    assert_eq!(
        relic_detail.amount.value().expect("relic amount"),
        &integer(4, "count")
    );
}
