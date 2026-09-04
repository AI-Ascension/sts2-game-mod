// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV3TransitionWitness(
    ulong FromGeneration,
    ulong ToGeneration,
    string StateId,
    string EffectKind);

internal static class PostconditionVerifier
{
    internal static bool Verify(
        RuntimeV3GameplayObservation before,
        RuntimeV3GameplayObservation after,
        RuntimeV3TransitionWitness witness,
        out string error)
    {
        if (!before.Validate(out error) || !after.Validate(out error))
        {
            return false;
        }
        if (after.Generation <= before.Generation
            || witness.FromGeneration != before.Generation
            || witness.ToGeneration != after.Generation
            || !string.Equals(witness.StateId, after.StateId, StringComparison.Ordinal)
            || !RuntimeV3GameplayContract.IsIdentity(witness.StateId)
            || !RuntimeV3GameplayContract.IsIdentity(witness.EffectKind))
        {
            error = "settlement lacks an independently verified fresh transition";
            return false;
        }
        error = string.Empty;
        return true;
    }
}
