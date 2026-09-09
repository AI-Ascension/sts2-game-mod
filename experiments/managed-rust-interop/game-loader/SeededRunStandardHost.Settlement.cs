// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class SeededRunStandardHost
{
    private static void Pump(PendingOperation pending)
    {
        if (!ReferenceEquals(_active, pending)
            || pending.Receipt.Status == SeededRunStandardHostStatus.Settled
            || pending.Receipt.Status == SeededRunStandardHostStatus.Rejected
            || pending.Receipt.Status == SeededRunStandardHostStatus.Unknown
                && !pending.AllowLateReadback)
        {
            return;
        }

        if (TryReadback(pending, out ReadbackResult result))
        {
            if (result.Settled)
            {
                Settle(pending, result.CanonicalSeed!, result.Compatibility!, result.Generation);
            }
            else
            {
                MarkUnknown(pending, result.ErrorCode!, result.Retryable);
            }

            return;
        }

        if (Elapsed(pending.StartedAt) >= StartTimeout)
        {
            MarkUnknown(pending, "run_start_timeout", allowLateReadback: true);
        }
    }

    private static bool TryReadback(PendingOperation pending, out ReadbackResult result)
    {
        result = default;
        try
        {
            RunManager manager = RunManager.Instance;
            // The pre-admission guard requires a null state. Read the first state as soon as the
            // native transition exposes it; IsInProgress and the run node settle later.
            RunState? state = manager.DebugOnlyGetState();
            if (state is null)
            {
                return false;
            }

            if (pending.StartedState is null)
            {
                // SetReady starts the native transition synchronously, but RunState is assigned
                // only after that transition yields. Bind the first non-null state observed by
                // the host pump before requiring a settled run node.
                pending.StartedState = state;
            }
            else if (!ReferenceEquals(state, pending.StartedState))
            {
                result = new ReadbackResult(false, null, null, 0,
                    "native_run_identity_mismatch", false);
                return true;
            }

            if (!manager.IsInProgress)
            {
                return false;
            }

            NRun? currentRunNode = NGame.Instance?.CurrentRunNode;
            if (pending.StartedRunNode is null && currentRunNode is not null)
            {
                pending.StartedRunNode = currentRunNode;
            }

            if (currentRunNode is null)
            {
                return false;
            }

            if (pending.StartedRunNode is { } startedRunNode
                && !ReferenceEquals(currentRunNode, startedRunNode))
            {
                result = new ReadbackResult(false, null, null, 0,
                    "native_run_node_mismatch", false);
                return true;
            }

            if (manager.ShouldSave
                && state.GameMode == GameMode.Standard
                && state.AscensionLevel == pending.Request.SelectedContext.Ascension
                && state.Rng is not null
                && state.Rng.StringSeed == pending.CanonicalSeed
                && manager.DailyTime is null
                && manager.NetService.Type == NetGameType.Singleplayer
                && state.Players.Count == 1
                && state.Players[0].Character.Id == ModelDb.Character<Ironclad>().Id
                && state.Modifiers.Count == pending.Request.SelectedContext.Modifiers.Count
                && state.Modifiers.Select(modifier => modifier.Id.Entry)
                    .SequenceEqual(pending.Request.SelectedContext.Modifiers, StringComparer.Ordinal)
                && state.Acts.Select(act => act.Id.Entry)
                    .SequenceEqual(pending.Request.SelectedContext.Acts, StringComparer.Ordinal)
                && SaveManager.Instance.Progress.NumberOfRuns == 0)
            {
                if (!LiveCombatSource.TryReadCurrentGeneration(out ulong generation))
                {
                    result = new ReadbackResult(false, null, null, 0,
                        "runtime_generation_unavailable", true);
                    return true;
                }

                if (generation <= pending.GenerationBefore)
                {
                    result = new ReadbackResult(false, null, null, generation,
                        "runtime_generation_stale", true);
                    return true;
                }

                SeededRunStandardCompatibilitySnapshot currentCompatibility =
                    SeededRunStandardCompatibilitySnapshot.Current();
                if (!currentCompatibility.Equals(pending.Compatibility))
                {
                    result = new ReadbackResult(false, null, null, generation,
                        "compatibility_changed_during_start", false);
                    return true;
                }

                result = new ReadbackResult(true, pending.CanonicalSeed,
                    currentCompatibility, generation, null, false);
                return true;
            }

            // A live RunState with concrete but different fields means the native operation has
            // taken effect in an unexpected context. It is unknown and must never be retried.
            result = new ReadbackResult(false, null, null, 0,
                "native_readback_mismatch", false);
            return true;
        }
        catch (Exception)
        {
            // The host may expose a partially initialized RunState for a few frames. Keep waiting;
            // a timeout becomes unknown and never cleans up or retries the run.
            return false;
        }
    }

    private static void Settle(
        PendingOperation pending,
        string canonicalSeed,
        SeededRunStandardCompatibilitySnapshot compatibility,
        ulong generation)
    {
        var observation = new SeededRunStandardObservation(
            RunStarted: true,
            HostReady: NGame.Instance is not null && RunManager.Instance.IsInProgress,
            Generation: generation,
            CanonicalSeed: canonicalSeed,
            SelectedContextDigest: pending.Request.SelectedContext.ContextDigest,
            PhaseBefore: PhaseBefore,
            PhaseAfter: PhaseAfter,
            CompatibilityIdentity: compatibility.CombinedIdentity);
        var witness = new SeededRunStandardEffectWitness(
            RunStartedWitness, generation, canonicalSeed);
        pending.Receipt = pending.Receipt with
        {
            Status = SeededRunStandardHostStatus.Settled,
            CanonicalSeed = canonicalSeed,
            Observation = observation,
            EffectWitness = witness,
            ErrorCode = null
        };
        if (ReferenceEquals(_active, pending))
        {
            _active = null;
        }
    }

    private static SeededRunStandardHostReceipt MarkRejected(
        PendingOperation pending,
        string errorCode)
    {
        pending.Receipt = pending.Receipt with
        {
            Status = SeededRunStandardHostStatus.Rejected,
            CanonicalSeed = null,
            Observation = null,
            EffectWitness = null,
            ErrorCode = errorCode
        };
        if (ReferenceEquals(_active, pending))
        {
            _active = null;
        }

        return pending.Receipt;
    }

    private static SeededRunStandardHostReceipt MarkUnknown(
        PendingOperation pending,
        string errorCode,
        bool allowLateReadback = true)
    {
        pending.Receipt = pending.Receipt with
        {
            Status = SeededRunStandardHostStatus.Unknown,
            CanonicalSeed = null,
            Observation = null,
            EffectWitness = null,
            ErrorCode = errorCode
        };
        pending.AllowLateReadback = allowLateReadback;
        if (ReferenceEquals(_active, pending) && !allowLateReadback)
        {
            _active = null;
        }

        return pending.Receipt;
    }
}
