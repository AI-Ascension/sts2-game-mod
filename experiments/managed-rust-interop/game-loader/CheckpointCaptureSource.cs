// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Collections.Immutable;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Host-thread-only checkpoint capture seam (issue #80 item 3, managed half). It classifies host
/// settlement, and only when <see cref="CheckpointSettlement.Quiescent"/> copies the payload-v1
/// families into owned records and immutable bytes. It draws no RNG, advances no observation
/// generation, mutates nothing, and holds no host reference after returning. It is not wired to
/// any route or listener: every boundary stays unavailable until exact-host evidence exists.
/// </summary>
internal sealed partial class CheckpointCaptureSource
{
    private readonly int _threadId = Environment.CurrentManagedThreadId;
    private readonly ICheckpointCaptureHostReader _host;
    private bool _inCaptureWindow;

    internal CheckpointCaptureSource(ICheckpointCaptureHostReader host)
    {
        ArgumentNullException.ThrowIfNull(host);
        _host = host;
    }

    /// <summary>The complete fail-closed matrix; identical dispositions to the Rust owner.</summary>
    internal static IReadOnlyList<CheckpointCaptureCapability> Capabilities()
    {
        var capabilities = new CheckpointCaptureCapability[CheckpointCaptureBoundaries.All.Count];
        for (int index = 0; index < capabilities.Length; index++)
            capabilities[index] = CheckpointCaptureBoundaries.Capability(
                CheckpointCaptureBoundaries.All[index]);
        return capabilities;
    }

    /// <summary>
    /// Captures one boundary on the host thread. The order is fixed: thread guard, boundary
    /// matrix, window exclusivity, settlement classification, family copy, settlement re-read,
    /// coverage rules, serialization. A rejection never carries an artifact.
    /// </summary>
    internal CheckpointCaptureOutcome Capture(CheckpointCaptureBoundary boundary)
    {
        RequireThread();
        CheckpointCaptureCapability capability = CheckpointCaptureBoundaries.Capability(boundary);
        if (!capability.HasPayloadSchema)
        {
            CheckpointCaptureRejectionKind kind =
                capability.Reason == CheckpointUnavailableReason.UnsafeBoundary
                    ? CheckpointCaptureRejectionKind.UnsafeBoundary
                    : CheckpointCaptureRejectionKind.UnsupportedBoundary;
            return Reject(kind, boundary, CheckpointSettlement.Unknown, capability.Reason.Code());
        }

        if (_inCaptureWindow)
            return Reject(CheckpointCaptureRejectionKind.Busy, boundary,
                CheckpointSettlement.Unknown, "capture_window_held");

        _inCaptureWindow = true;
        try
        {
            CheckpointPayloadFamilies families;
            try
            {
                CheckpointHostSettlementWitness before = _host.ReadSettlement();
                CheckpointSettlementClassification classification = Classify(before, boundary);
                if (classification.Settlement != CheckpointSettlement.Quiescent)
                    return Reject(CheckpointCaptureRejectionKind.UnsafeBoundary, boundary,
                        classification.Settlement, classification.Detail);

                families = _host.ReadFamilies(boundary);
                if (before != _host.ReadSettlement())
                    return Reject(CheckpointCaptureRejectionKind.UnsafeBoundary, boundary,
                        CheckpointSettlement.Unknown, "settlement_changed_inside_window");
            }
            catch (Exception)
            {
                // Any host read that throws is an unclassifiable settlement, never a partial
                // capture. The thread guard runs before this window, so its typed refusal is
                // never swallowed here, and everything below this point reads owned records only.
                return Reject(CheckpointCaptureRejectionKind.UnsafeBoundary, boundary,
                    CheckpointSettlement.Unknown, "host_read_failed");
            }

            if (!TryValidate(boundary, families, out CheckpointCaptureRejection? rejection))
                return CheckpointCaptureOutcome.Rejected(rejection);

            var record = new CheckpointPayloadRecord(boundary, families);
            ImmutableArray<byte> bytes = CheckpointCapturePayloadWriter.Write(record);
            return CheckpointCaptureOutcome.Captured(record, bytes);
        }
        finally
        {
            _inCaptureWindow = false;
        }
    }

    private void RequireThread()
    {
        if (Environment.CurrentManagedThreadId != _threadId)
            throw new InvalidOperationException("checkpoint capture requires the host thread");
    }

    private static CheckpointCaptureOutcome Reject(
        CheckpointCaptureRejectionKind kind,
        CheckpointCaptureBoundary boundary,
        CheckpointSettlement settlement,
        string detail) =>
        CheckpointCaptureOutcome.Rejected(
            new CheckpointCaptureRejection(kind, boundary, settlement, detail));
}
