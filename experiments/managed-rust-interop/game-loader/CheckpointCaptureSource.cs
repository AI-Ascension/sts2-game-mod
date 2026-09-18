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
            CheckpointHostSettlementWitness before = _host.ReadSettlement();
            CheckpointSettlementClassification classification = Classify(before, boundary);
            if (classification.Settlement != CheckpointSettlement.Quiescent)
                return Reject(CheckpointCaptureRejectionKind.UnsafeBoundary, boundary,
                    classification.Settlement, classification.Detail);

            CheckpointPayloadFamilies families = _host.ReadFamilies(boundary);
            CheckpointHostSettlementWitness after = _host.ReadSettlement();
            if (before != after)
                return Reject(CheckpointCaptureRejectionKind.UnsafeBoundary, boundary,
                    CheckpointSettlement.Unknown, "settlement_changed_inside_window");

            if (!TryValidate(boundary, families, out CheckpointCaptureRejection? rejection))
                return CheckpointCaptureOutcome.Rejected(rejection);

            var record = new CheckpointPayloadRecord(boundary, families);
            ImmutableArray<byte> bytes = CheckpointCapturePayloadWriter.Write(record);
            return CheckpointCaptureOutcome.Captured(record, bytes);
        }
        catch (Exception exception) when (exception is not InvalidOperationException)
        {
            // A host read that throws is an unclassifiable settlement, never a partial capture.
            return Reject(CheckpointCaptureRejectionKind.UnsafeBoundary, boundary,
                CheckpointSettlement.Unknown, "host_read_failed");
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
