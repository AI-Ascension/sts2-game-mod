// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using Godot;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Host-thread owner for the narrow standard seeded-run path.
/// It uses the native standard lobby and only the native debug seed override; it never calls the
/// custom-run entry point, writes RNG state, changes acts, or deletes saves.
/// </summary>
internal static partial class SeededRunStandardHost
{
    internal const string NativeSelectionPolicy = SeededRunSelectionContext.StandardDefaultSelectionPolicy;
    internal const string RunStartedWitness = "run_started";
    internal const string PhaseBefore = "campaign_setup";
    internal const string PhaseAfter = "run_started";

    private const int MaxReceipts = 4096;
    private static readonly TimeSpan StartTimeout = TimeSpan.FromSeconds(30);
    private static readonly Dictionary<string, PendingOperation> Operations = new(StringComparer.Ordinal);
    private static PendingOperation? _active;
    private static int? _hostThreadId;

    /// <summary>True while a start has been admitted but has no settled receipt.</summary>
    internal static bool HasPendingMutation => _active is not null;

    /// <summary>
    /// Admits one request on the Godot host thread. The native lobby begins asynchronously; callers
    /// must reconcile the operation to obtain a settled witness.
    /// </summary>
    internal static SeededRunStandardHostReceipt Start(
        SeededRunStandardRequest request,
        ulong requestGeneration = 0)
    {
        RequireHostThread();

        if (request is null)
        {
            return InvalidReceipt("", "", null, "invalid_request");
        }

        string operationId = request.OperationId ?? string.Empty;
        if (Operations.TryGetValue(operationId, out PendingOperation? existing))
        {
            string fingerprint = SafeFingerprint(request);
            if (IsExactReplay(existing.Fingerprint, fingerprint,
                    existing.Receipt.RequestGeneration, requestGeneration))
            {
                Pump(existing);
                return existing.Receipt;
            }

            return existing.Receipt with
            {
                Status = SeededRunStandardHostStatus.Rejected,
                CanonicalSeed = null,
                Observation = null,
                EffectWitness = null,
                ErrorCode = "idempotency_conflict"
            };
        }

        if (!request.Validate(out string requestError))
        {
            return RetainRejected(request, requestError == ""
                ? "invalid_request" : "invalid_context", requestGeneration);
        }

        if (_active is not null)
        {
            return RetainRejected(request, "operation_in_progress", requestGeneration);
        }

        if (Operations.Count >= MaxReceipts)
        {
            return AcceptedShape(request, requestGeneration) with
            {
                Status = SeededRunStandardHostStatus.Rejected,
                ErrorCode = "receipt_capacity_exhausted"
            };
        }

        if (!TryPrepare(request, out NCharacterSelectScreen screen,
                out string canonicalSeed, out SeededRunStandardCompatibilitySnapshot? compatibility,
                out ulong generationBefore, out string preparationError))
        {
            return RetainRejected(request, preparationError, requestGeneration);
        }

        var pending = new PendingOperation(
            request,
            request.Fingerprint(),
            AcceptedShape(request, requestGeneration),
            canonicalSeed,
            compatibility,
            screen,
            generationBefore,
            StopwatchTimestamp());
        Operations.Add(operationId, pending);
        _active = pending;

        try
        {
            CharacterModel ironclad = ModelDb.Character<Ironclad>();
            screen.Lobby.SetLocalCharacter(ironclad);
            if (screen.Lobby.LocalPlayer.character?.Id != ironclad.Id)
            {
                return MarkRejected(pending, "native_character_not_retained");
            }

            // SetReady is the mutation boundary. The host itself consumes DebugSeedOverride when
            // standard lobby BeginRunForAllPlayersIfAllReady chooses its canonical seed.
            NGame game = NGame.Instance!;
            game.DebugSeedOverride = canonicalSeed;
            try
            {
                pending.MutationAttempted = true;
                screen.Lobby.SetReady(ready: true);
            }
            finally
            {
                game.DebugSeedOverride = null;
            }
        }
        catch (Exception exception)
        {
            string code = pending.MutationAttempted
                ? "host_start_outcome_unknown"
                : "host_start_rejected";
            return pending.MutationAttempted
                ? MarkUnknown(pending, code)
                : MarkRejected(pending, code + ":" + exception.GetType().Name);
        }

        return pending.Receipt;
    }

    /// <summary>Reads and settles one retained operation without ever retrying its mutation.</summary>
    internal static SeededRunStandardHostReceipt Reconcile(string operationId)
    {
        RequireHostThread();
        if (!SeededRunStandardContract.IsIdentity(operationId)
            || !Operations.TryGetValue(operationId, out PendingOperation? pending))
        {
            return InvalidReceipt(operationId, "", null, "operation_not_found");
        }

        Pump(pending);
        return pending.Receipt;
    }

    /// <summary>Returns a retained receipt without advancing host state.</summary>
    internal static bool TryGetReceipt(
        string operationId,
        out SeededRunStandardHostReceipt receipt)
    {
        RequireHostThread();
        if (SeededRunStandardContract.IsIdentity(operationId)
            && Operations.TryGetValue(operationId, out PendingOperation? pending))
        {
            receipt = pending.Receipt;
            return true;
        }

        receipt = null!;
        return false;
    }

    /// <summary>Retains a protocol-valid request as rejected before mutation admission.</summary>
    internal static SeededRunStandardHostReceipt Reject(
        SeededRunStandardRequest request,
        string errorCode,
        ulong requestGeneration = 0)
    {
        RequireHostThread();
        return RetainRejected(request, errorCode, requestGeneration);
    }

    /// <summary>Allows a host frame pump to advance an admitted launch.</summary>
    internal static void Pump()
    {
        RequireHostThread();
        if (_active is not null)
        {
            Pump(_active);
        }
    }

    internal static SeededRunStandardCompatibilitySnapshot CurrentCompatibility()
    {
        RequireHostThread();
        return SeededRunStandardCompatibilitySnapshot.Current();
    }

}
