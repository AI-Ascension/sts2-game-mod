// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// The opaque half of the bounded <c>seeded-run-v1</c> identity contract, for the slots an operator
/// supplies. The pinned schema defines one <c>identity</c> pattern,
/// <c>^[A-Za-z0-9_.:/-]{1,128}$</c>, and every identity-bearing slot refers to it. That alphabet
/// is owner-defined in <c>sts2-protocol</c> and mirrored byte-identically into the consumers, so it
/// is not this repository's to narrow: the release-like compatibility identities
/// (<c>sts2-game/v0.107.1</c>) and the composite <c>context_id</c>
/// (<c>standard/ironclad/asc0/fresh</c>) need its <c>/</c> and <c>:</c>.
/// </summary>
/// <remarks>
/// What that alphabet also spells is a POSIX or Windows host path, because <c>/</c>, <c>.</c>, and
/// <c>-</c> together are enough. The runtime identifiers and the profile-baseline identity never
/// legitimately carry one, so they take this stricter class: the shared alphabet, minus the
/// structural sequences that only occur in paths and URIs. This mirrors
/// <c>is_opaque_identity</c> in the seeded-run client and the save-profile identity check in
/// <c>sts2-gateway</c>. It is a producer-side refinement, so it changes no schema, digest, or
/// golden, and it is a strict subset of <see cref="SeededRunStandardContract.IsIdentity"/> — a
/// value accepted here is one the shared pattern also accepts, so the two sides cannot drift on a
/// legitimate value.
/// </remarks>
internal static class SeededRunOpaqueIdentity
{
    /// <summary>
    /// Bounds an operator-supplied runtime identity. <c>:</c> stays admissible so a
    /// colon-delimited identifier such as the co-op producer's <c>instance:native-test</c> keeps
    /// working, but a leading colon, a leading separator, or a parent-directory hop does not.
    /// </summary>
    internal static bool IsOpaqueIdentity(string? value)
    {
        // `IsIdentity` is itself null-tolerant, but the compiler cannot see through that, and the
        // negative call does not narrow `value` for the reader below. Bind the validated form once
        // so the checks that dereference cannot dereference null, and so the narrowing is a fact of
        // this method rather than of the flow analyser's mood.
        if (!SeededRunStandardContract.IsIdentity(value) || value is null)
        {
            return false;
        }

        return !value.StartsWith('/')
            && !value.StartsWith('\\')
            && !value.Contains("..", StringComparison.Ordinal)
            && !value.Contains("://", StringComparison.Ordinal)
            && !HasDriveLetterPrefix(value)
            && value.Trim('/').Length != 0;
    }

    /// <summary>
    /// Refuses a bare drive prefix such as <c>C:</c> or <c>Z:\profiles</c> while leaving a
    /// colon-delimited identifier such as <c>instance:native-test</c> admissible. A <c>:</c> with
    /// no letter before it is a scheme or drive separator with its prefix missing, which is the
    /// same shape one hop further along, so it is refused too: only a colon that has an opaque
    /// identifier <em>before</em> it is admissible. A bare <c>.</c> or <c>./</c> is admitted by
    /// decision: it names the current directory rather than a host location, so it discloses
    /// nothing, and refusing it would narrow a producer-owned alphabet this side does not own.
    /// Mirrors <c>is_opaque_identity</c> in the seeded-run client.
    /// </summary>
    private static bool HasDriveLetterPrefix(string value) =>
        value.Length >= 1
        && (value[0] == ':'
            || (value.Length >= 2 && value[1] == ':' && char.IsAsciiLetter(value[0])));
}
