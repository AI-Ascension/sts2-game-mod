// SPDX-License-Identifier: MIT

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::context::{CanonicalContext, Compatibility, IdentityDigest, ProfileBaseline};
use crate::request::{StartRequestParams, build_start_request};

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn sample_context() -> CanonicalContext {
    CanonicalContext {
        context_id: "ctx-1".to_owned(),
        game_mode: "standard".to_owned(),
        character: "ironclad".to_owned(),
        ascension: 0,
        modifiers: Vec::new(),
        acts: vec!["act-1".to_owned()],
        selection_policy: "standard_default".to_owned(),
        profile_baseline: ProfileBaseline {
            kind: "fresh".to_owned(),
            identity: "prof-1".to_owned(),
            digest: digest('a'),
        },
        save_policy: "enabled".to_owned(),
        compatibility: Compatibility {
            game: IdentityDigest {
                identity: "sts2/1.0.0.0".to_owned(),
                digest: digest('b'),
            },
            mod_identity: IdentityDigest {
                identity: "AIAscensionSTS2GameMod/1.0.0.0".to_owned(),
                digest: digest('c'),
            },
        },
    }
}

fn sample_params() -> StartRequestParams {
    StartRequestParams {
        schema_digest: digest('d'),
        correlation_id: "corr-1".to_owned(),
        instance_id: "inst-1".to_owned(),
        session_id: "sess-1".to_owned(),
        lease_id: "lease-1".to_owned(),
        lease_epoch: 1,
        generation: 0,
        operation_id: "op-1".to_owned(),
        requested_seed: "seed-1".to_owned(),
        run_mode: "seeded_training".to_owned(),
        context: sample_context(),
    }
}

#[test]
fn canonical_digest_matches_contract_vector() {
    assert_eq!(
        sample_context().digest().unwrap(),
        "f9cc2c6c60624cb1da738999b3851dcd1bc1fdd15881bcaf8306171a9b4c660b"
    );
}

#[test]
fn request_carries_digest_and_required_fields() {
    let request = build_start_request(&sample_params()).unwrap();
    let json = request.to_json().unwrap();
    for key in [
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "lease_epoch",
        "generation",
        "kind",
        "operation_id",
        "requested_seed",
        "run_mode",
        "context_digest",
        "selected_context",
        "status",
        "canonical_seed",
        "observation",
        "effect_witness",
        "error_code",
    ] {
        assert!(json.contains(&format!("\"{key}\"")), "missing {key}");
    }
    assert!(json.contains("\"status\":null"));
    assert!(json.contains("\"kind\":\"start_request\""));
    assert!(json.contains(&format!(
        "\"context_digest\":\"{}\"",
        request.context_digest
    )));
}

#[test]
fn validation_rejects_out_of_contract_inputs() {
    let mut context = sample_context();
    context.ascension = 21;
    assert!(context.validate().is_err());

    let mut context = sample_context();
    context.modifiers = vec!["b".to_owned(), "a".to_owned()];
    assert!(context.validate().is_err());

    let mut context = sample_context();
    context.game_mode = "daily".to_owned();
    assert!(context.validate().is_err());
}

#[test]
fn request_rejects_invalid_seed_and_identity() {
    let mut params = sample_params();
    params.requested_seed = String::new();
    assert!(build_start_request(&params).is_err());

    let mut params = sample_params();
    params.operation_id = "bad id".to_owned();
    assert!(build_start_request(&params).is_err());

    let mut params = sample_params();
    params.schema_digest = "nothex".to_owned();
    assert!(build_start_request(&params).is_err());
}

/// The redaction half of the bounded-probe criterion, pinned for the seeded-run request surface.
///
/// #79's first acceptance criterion asks for "argument/authority **and data-redaction** checks"
/// that run without a game. The argument side is covered by the tests above. This covers the
/// other half: no operator-supplied field may carry a filesystem path or an e-mail address into
/// the request body.
///
/// The threat is concrete rather than theoretical. The pinned `seeded-run-v1` identity pattern
/// admits `.`, `:`, `/` and `-`, which the contract needs for identities like
/// `sts2-game/v0.107.1` and for the composite `context_id` `standard/ironclad/asc0/fresh`. Those
/// same characters spell a POSIX path or a Windows one. So an operator who pastes
/// `instance_id = "/home/operator/sts2/profiles/slot1"` today gets a request that validates cleanly
/// and carries a personal host path onto the wire and into whatever run record the host keeps.
///
/// Each assertion below is a refusal that must hold. They are written as `is_err()` over a
/// *sentinel that must not appear in an accepted body*, and each is paired with an accepted
/// control, so the refusals cannot be satisfied by a client that simply refuses everything.
#[test]
fn no_operator_field_carries_a_host_path_or_email_into_the_body() {
    const SENTINEL: &str = "private-sentinel";

    // A host path in each operator-supplied runtime identity slot, reached by more than one route.
    for (field, host_path) in [
        (
            "instance_id",
            format!("/home/{SENTINEL}/sts2/profiles/slot1"),
        ),
        ("session_id", format!("/home/{SENTINEL}/sts2/sessions/a")),
        ("lease_id", format!("/var/lib/{SENTINEL}/leases/l1")),
        ("correlation_id", format!("/home/{SENTINEL}/traces/c1")),
        // Parent-directory hops and scheme separators spell the same thing.
        ("instance_id", format!("profiles/../../home/{SENTINEL}")),
        ("instance_id", format!("file:///home/{SENTINEL}/profile")),
        // A Windows drive path and a UNC path, which the shared alphabet also spells.
        ("instance_id", format!("C:\\Users\\{SENTINEL}\\profile")),
        ("instance_id", format!("\\\\host\\share\\{SENTINEL}")),
        // A bare drive prefix and a bare scheme separator, neither of which has a leading
        // separator, a `..`, or a `://` for the structural rules above to catch.
        ("instance_id", format!("{SENTINEL}:\\profiles")),
        ("instance_id", format!(":{SENTINEL}")),
    ] {
        let mut params = sample_params();
        match field {
            "instance_id" => params.instance_id = host_path.clone(),
            "session_id" => params.session_id = host_path.clone(),
            "lease_id" => params.lease_id = host_path.clone(),
            _ => params.correlation_id = host_path.clone(),
        }
        assert!(
            build_start_request(&params).is_err(),
            "a host path in {field} must be refused, not serialized: {host_path}"
        );
    }

    // A separator-only value is a path with no name, so it carries no identity at all.
    let mut params = sample_params();
    params.lease_id = "/".to_owned();
    assert!(
        build_start_request(&params).is_err(),
        "a separator-only value must be refused"
    );

    // The profile-baseline identity is operator-supplied too, and it is the slot most likely to
    // be pasted from a real profile directory.
    let mut params = sample_params();
    params.context.profile_baseline.identity = format!("/home/{SENTINEL}/sts2/profiles/slot1");
    assert!(
        build_start_request(&params).is_err(),
        "a host path in profile_baseline.identity must be refused"
    );

    // An e-mail address is the other shape a personal identifier arrives in. `@` is outside the
    // identity alphabet, so this must already be refused; the assertion pins that it stays so.
    let mut params = sample_params();
    params.context.profile_baseline.identity = format!("{SENTINEL}@example.com");
    assert!(
        build_start_request(&params).is_err(),
        "an e-mail address in profile_baseline.identity must be refused"
    );

    // Control 1: the identities the contract actually admits must still build, and the sentinel
    // must not survive anywhere in an accepted body.
    let accepted = build_start_request(&sample_params())
        .expect("the contract identities still build")
        .to_json()
        .expect("a built request encodes");
    assert!(
        !accepted.contains(SENTINEL),
        "no sentinel value may appear in an accepted body"
    );

    // Control 2: a colon-delimited runtime identifier is a real in-tree shape (the co-op native
    // producer uses `instance:native-test`), so the refinement must not over-reach and break it.
    let mut params = sample_params();
    params.instance_id = "instance:native-test".to_owned();
    assert!(
        build_start_request(&params).is_ok(),
        "a colon-delimited runtime identity stays admitted"
    );

    // Control 3: the artifact-identity class is untouched by the runtime refinement and must keep
    // the `name/version` shape that the pinned contract and its goldens depend on. The composite
    // `context_id` is asserted against the value the contract goldens actually carry, not against
    // the local sample, so this control fails if the `/`-bearing identity class is ever narrowed.
    let mut composite = sample_params();
    composite.context.context_id = "standard/ironclad/asc0/fresh".to_owned();
    assert!(
        build_start_request(&composite).is_ok(),
        "the composite `name/version` context_id stays admitted"
    );
    for identity in [
        &sample_context().compatibility.game.identity,
        &sample_context().compatibility.mod_identity.identity,
    ] {
        assert!(
            identity.contains('/') && !identity.contains(' '),
            "the control relies on the contract's `name/version` identity shape staying admitted: \
             {identity}"
        );
    }
}
