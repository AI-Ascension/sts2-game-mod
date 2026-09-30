// SPDX-License-Identifier: MIT

//! Bounded identity grammars for the pinned `seeded-run-v1` contract.
//!
//! The contract defines a single `identity` definition, `^[A-Za-z0-9_.:/-]{1,128}$`, and every
//! identity-bearing slot — runtime, context, and release-like — refers to it. That alphabet is
//! owner-defined in `sts2-protocol` and mirrored byte-identically into the consumers, so it is not
//! this crate's to narrow. It must keep `/` and `:`: the release-like identities are
//! `sts2-game/v0.107.1` and `ai-ascension/sts2-game-mod`, and the composite `context_id` is
//! `standard/ironclad/asc0/fresh`.
//!
//! What that alphabet also spells is a filesystem path, because `/`, `.`, and `-` together are
//! enough. So this module splits one shared definition into the two classes the values actually
//! form, and applies the stricter one only where a path is never a legitimate value. That keeps
//! the redaction property without a cross-repository schema, digest, and golden change.

/// Maximum length shared with the `seeded-run-v1` `identity` schema pattern.
const MAX_IDENTITY_BYTES: usize = 128;

/// Bounded artifact-identity grammar for `seeded-run-v1`.
///
/// This is the pinned wire alphabet, retained verbatim. Use it for the release-like identities the
/// contract carries — compatibility `name/version` values, the composite `context_id`, error
/// codes, and the observation phase identities — all of which legitimately contain `/`.
#[must_use]
pub fn is_artifact_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTITY_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".:/_-".contains(&byte))
}

/// Bounded opaque-identity grammar for the operator-supplied runtime identifiers.
///
/// `instance_id`, `session_id`, `lease_id`, `correlation_id`, `operation_id`, and the
/// `profile_baseline` identity name a live routed operation or one prepared profile. They are
/// never a path, a URI, or a release-like `name/version`, so they take this stricter class rather
/// than the shared one.
///
/// The refinement keeps the contract alphabet and refuses the structural sequences that only occur
/// in paths and URIs — the same shape `sts2-gateway` applies to save-profile identities in
/// `save_profile/types.rs`. Without it, an operator who supplies
/// `instance_id = "/home/operator/sts2/profiles/slot1"` gets a request that validates cleanly and
/// carries a personal host path onto the wire and into whatever run record the host keeps.
///
/// This is a strict subset of [`is_artifact_identity`], which matters for the contract: the
/// consumer in the C# loader mirrors this predicate, so a value it accepts is one the shared
/// schema pattern also accepts and the two sides cannot drift on a legitimate value.
///
/// `:` stays admissible so a colon-delimited runtime identifier such as the co-op native
/// producer's `instance:native-test` keeps working; a colon may not be the leading character,
/// which is the shape of a bare scheme separator, and a drive prefix such as `C:` or `Z:\profiles`
/// is refused outright. `/` stays admissible for the same reason `context_id` keeps it, so the
/// two classes are complementary rather than redundant.
#[must_use]
pub fn is_opaque_identity(value: &str) -> bool {
    is_artifact_identity(value)
        // A POSIX absolute path, a Windows drive or UNC prefix, or a parent-directory hop.
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains("..")
        && !value.contains("://")
        // A bare scheme separator carries no authority value.
        && !matches!(value.as_bytes().first(), Some(b':'))
        // A drive prefix such as `C:` or `Z:\profiles` is a path root, not an identity.
        && !has_drive_letter_prefix(value)
        // A value made only of separators is a path with no name, not an identity.
        && !value.trim_matches('/').is_empty()
}

/// Whether `value` opens with a bare ASCII drive prefix, as in `C:` or `Z:\profiles`.
fn has_drive_letter_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    matches!(bytes, [drive, b':', ..] if drive.is_ascii_alphabetic())
}
