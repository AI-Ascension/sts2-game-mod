// SPDX-License-Identifier: MIT

using System;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class SeededRunStandardHost
{
    private static SeededRunStandardHostReceipt RetainRejected(
        SeededRunStandardRequest request,
        string errorCode,
        ulong requestGeneration = 0)
    {
        SeededRunStandardHostReceipt receipt = AcceptedShape(request, requestGeneration) with
        {
            Status = SeededRunStandardHostStatus.Rejected,
            ErrorCode = errorCode
        };
        if (SeededRunStandardContract.IsIdentity(request.OperationId)
            && Operations.Count < MaxReceipts)
        {
            Operations[request.OperationId] = new PendingOperation(
                request,
                SafeFingerprint(request),
                receipt,
                string.Empty,
                null,
                null,
                0,
                StopwatchTimestamp());
        }

        return receipt;
    }

    private static SeededRunStandardHostReceipt AcceptedShape(
        SeededRunStandardRequest request,
        ulong requestGeneration = 0) =>
        new(request.OperationId, request.RequestedSeed, request.RunMode,
            request.SelectedContext, SeededRunStandardHostStatus.Accepted,
            null, null, null, null, requestGeneration);

    private static SeededRunStandardHostReceipt InvalidReceipt(
        string operationId,
        string requestedSeed,
        SeededRunSelectionContext? context,
        string errorCode,
        ulong requestGeneration = 0) =>
        new(operationId, requestedSeed, "diagnostic", context,
            SeededRunStandardHostStatus.Rejected, null, null, null, errorCode, requestGeneration);

    private static string SafeFingerprint(SeededRunStandardRequest request)
    {
        try
        {
            return request.Fingerprint();
        }
        catch
        {
            return string.Empty;
        }
    }

    private static void RequireHostThread()
    {
        int current = System.Environment.CurrentManagedThreadId;
        int? bound = _hostThreadId;
        if (bound is null)
        {
            _hostThreadId = current;
            bound = current;
        }

        if (bound != current)
        {
            throw new InvalidOperationException("seeded-run host access must stay on the Godot host thread");
        }
    }

    private static long StopwatchTimestamp() => System.Diagnostics.Stopwatch.GetTimestamp();

    private static TimeSpan Elapsed(long started) =>
        System.Diagnostics.Stopwatch.GetElapsedTime(started);

    private sealed class PendingOperation
    {
        internal PendingOperation(
            SeededRunStandardRequest request,
            string fingerprint,
            SeededRunStandardHostReceipt receipt,
            string canonicalSeed,
            SeededRunStandardCompatibilitySnapshot? compatibility,
            NCharacterSelectScreen? screen,
            ulong generationBefore,
            long startedAt)
        {
            Request = request;
            Fingerprint = fingerprint;
            Receipt = receipt;
            CanonicalSeed = canonicalSeed;
            Compatibility = compatibility;
            Screen = screen;
            GenerationBefore = generationBefore;
            StartedAt = startedAt;
        }

        internal SeededRunStandardRequest Request { get; }
        internal string Fingerprint { get; }
        internal SeededRunStandardHostReceipt Receipt { get; set; }
        internal string CanonicalSeed { get; }
        internal SeededRunStandardCompatibilitySnapshot? Compatibility { get; }
        internal NCharacterSelectScreen? Screen { get; }
        internal ulong GenerationBefore { get; }
        internal long StartedAt { get; }
        internal bool MutationAttempted { get; set; }
        internal bool AllowLateReadback { get; set; }
        internal RunState? StartedState { get; set; }
        internal NRun? StartedRunNode { get; set; }
    }

    private readonly record struct ReadbackResult(
        bool Settled,
        string? CanonicalSeed,
        SeededRunStandardCompatibilitySnapshot? Compatibility,
        ulong Generation,
        string? ErrorCode,
        bool Retryable);
}
