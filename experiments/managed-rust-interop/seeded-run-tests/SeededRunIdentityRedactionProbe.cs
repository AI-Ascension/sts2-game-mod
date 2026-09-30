// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Pins the loader-side half of the bounded-probe redaction contract for
/// <c>sts2-game-mod#79</c>. The pinned <c>seeded-run-v1</c> <c>identity</c> pattern admits
/// <c>. : / -</c> because release-like identities and the composite <c>context_id</c> need them.
/// Those same characters spell a POSIX or Windows host path, so the operator-supplied runtime
/// slots take the stricter opaque grammar. This probe runs in the managed source-only CI lane and
/// needs no game, host assembly, or <c>STS2GameDataDir</c>.
/// </summary>
internal static class SeededRunIdentityRedactionProbe
{
    private static void Main()
    {
        // Host paths and URIs must never reach a run record through an identity slot.
        foreach (string value in new[]
        {
            "/home/operator/sts2/profiles/slot1",
            "/var/lib/sts2/leases/l1",
            "profiles/../../home/operator",
            "file:///home/operator/profile",
            "C:\\Users\\operator\\profile",
            "\\\\host\\share\\profile",
            // A bare drive prefix and a bare scheme separator: neither has a leading separator,
            // a ".." or a "://" for the structural rules to catch.
            "Z:\\profiles",
            "Z:",
            ":operator",
            "..",
            "../..",
            "/",
            "///",
        })
        {
            Check(!SeededRunOpaqueIdentity.IsOpaqueIdentity(value),
                "host path refused: " + value);
        }

        // An e-mail address is the other shape a personal identifier arrives in.
        Check(!SeededRunOpaqueIdentity.IsOpaqueIdentity("operator@example.com"),
            "e-mail address refused in a runtime identity");

        // Accepted controls. A colon-delimited runtime identifier is a real in-tree shape (the
        // co-op native producer uses "instance:native-test"), and an embedded slash is admitted
        // for the same reason the composite context_id keeps one.
        foreach (string value in new[]
        {
            "instance-1",
            "session-1",
            "lease-1",
            "corr-seed-0001",
            "op-seed-1",
            "instance:native-test",
            "tenant/instance-1",
            // A bare `.` and `./` are current-directory path elements, and they are admitted by
            // decision rather than by oversight. Neither names a host location, so neither
            // discloses anything, and refusing them would narrow a producer-owned alphabet this
            // consumer does not own. They are weak identities, not leaks, and pinning them here
            // keeps the decision from being silently reversed.
            ".",
            "./",
        })
        {
            Check(SeededRunOpaqueIdentity.IsOpaqueIdentity(value),
                "runtime identity admitted: " + value);
        }

        // The artifact class is unchanged, so the release-like identities the pinned contract and
        // its goldens depend on keep building. If this is ever narrowed the probe fails here
        // rather than at a native host that no default-CI lane can reach.
        foreach (string value in new[]
        {
            "sts2/0.1.0.0",
            "sts2-game/v0.107.1",
            "ai-ascension/sts2-game-mod",
            "standard/ironclad/asc0/fresh",
            "fresh-standard-comparison",
        })
        {
            Check(SeededRunStandardContract.IsIdentity(value),
                "artifact identity admitted: " + value);
        }

        // The opaque class is a strict subset of the artifact class: nothing it accepts may be
        // rejected by the shared alphabet, so the two cannot drift into a producer/consumer
        // mismatch on a legitimate value.
        foreach (string value in new[]
        {
            "instance-1", "instance:native-test", "tenant/instance-1", "op-seed-1",
        })
        {
            Check(SeededRunOpaqueIdentity.IsOpaqueIdentity(value)
                && SeededRunStandardContract.IsIdentity(value),
                "opaque acceptance implies artifact acceptance: " + value);
        }

        Console.WriteLine("seeded-run identity redaction checks passed");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }

        Console.WriteLine("PASS: " + message);
    }
}
