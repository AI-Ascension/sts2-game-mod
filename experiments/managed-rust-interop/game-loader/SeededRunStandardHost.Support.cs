// SPDX-License-Identifier: MIT

using System;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class SeededRunStandardHost
{
    private static SeededRunStandardHostReceipt RetainRejected(
        SeededRunStandardRequest request,
        string errorCode)
    {
        SeededRunStandardHostReceipt receipt = AcceptedShape(request) with
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

    private static SeededRunStandardHostReceipt AcceptedShape(SeededRunStandardRequest request) =>
        new(request.OperationId, request.RequestedSeed, request.RunMode,
            request.SelectedContext, SeededRunStandardHostStatus.Accepted,
            null, null, null, null);

    private static SeededRunStandardHostReceipt InvalidReceipt(
        string operationId,
        string requestedSeed,
        SeededRunSelectionContext? context,
        string errorCode) =>
        new(operationId, requestedSeed, "diagnostic", context,
            SeededRunStandardHostStatus.Rejected, null, null, null, errorCode);

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
    }

    private readonly record struct ReadbackResult(
        bool Settled,
        string? CanonicalSeed,
        SeededRunStandardCompatibilitySnapshot? Compatibility,
        ulong Generation,
        string? ErrorCode,
        bool Retryable);
}
