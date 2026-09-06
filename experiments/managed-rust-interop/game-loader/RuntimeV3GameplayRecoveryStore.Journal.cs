// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplayRecoveryStore
{
    private bool Transition(
        RuntimeV3OperationKey operation,
        RuntimeV3HostOperationState state,
        RuntimeV3DispatchStatus status,
        bool wasDispatched,
        string? errorCode,
        RuntimeV3HostTicketState ticketState,
        RuntimeV3TransitionWitness? witness,
        RuntimeV3GameplayObservation? observation,
        out string error)
    {
        lock (_gate)
        {
            error = string.Empty;
            if (!_healthy || _disposed)
            {
                error = "persistence_unavailable";
                return false;
            }
            if (!_operations.TryGetValue(operation, out RuntimeV3HostOperationRecord? existing))
            {
                error = "operation_not_found";
                return false;
            }
            if (existing.Status is RuntimeV3DispatchStatus.Settled
                or RuntimeV3DispatchStatus.Rejected)
            {
                error = "invalid_transition";
                return false;
            }
            if (state == RuntimeV3HostOperationState.Rejected && existing.WasDispatched)
            {
                error = "uncertainty_must_be_reconciled";
                return false;
            }
            if (state == RuntimeV3HostOperationState.Settled && !existing.WasDispatched
                && !wasDispatched)
            {
                error = "invalid_transition";
                return false;
            }
            if (errorCode is not null && !RuntimeV3GameplayContract.IsIdentity(errorCode))
            {
                error = "invalid_error_code";
                return false;
            }
            RuntimeV3HostOperationRecord updated = existing with
            {
                RecoveryState = state,
                Status = status,
                WasDispatched = existing.WasDispatched || wasDispatched,
                Ticket = existing.Ticket is null ? null : existing.Ticket with { State = ticketState },
                Observation = observation ?? existing.Observation,
                Witness = witness ?? existing.Witness,
                ErrorCode = errorCode,
                UpdatedAt = _clock().ToUniversalTime()
            };
            if (!AppendOperationLocked(updated))
            {
                error = "persistence_unavailable";
                return false;
            }
            _operations[operation] = updated;
            return true;
        }
    }

    private string ValidateBootstrapLocked(RuntimeV3HostBootstrapRequest request)
    {
        if (!_healthy || _disposed)
        {
            return "persistence_unavailable";
        }
        if (request.Contract != RuntimeV3GameplayRecoveryContract.Contract
            || request.SchemaDigest != RuntimeV3GameplayRecoveryContract.SchemaDigest
            || request.Release.RuntimeV3SchemaDigest != RuntimeV3GameplayContract.SchemaDigest
            || !RuntimeV3GameplayRecoveryContract.IsDigest(request.Release.ReleaseDigest)
            || !RuntimeV3GameplayRecoveryContract.IsDigest(request.Release.ConfigDigest)
            || !RuntimeV3GameplayRecoveryContract.IsDigest(request.Release.ProfileDigest)
            || !RuntimeV3GameplayRecoveryContract.IsDigest(request.Release.RuntimeV3SchemaDigest)
            || !RuntimeV3GameplayRecoveryContract.VerifyProof(
                _credentials.BootstrapSecret,
                "bootstrap",
                FenceBytes(request.Fence, request.Release),
                request.Proof))
        {
            return "auth_required";
        }

        RuntimeV3HostFence fence = request.Fence;
        if (!ValidFence(fence) || fence.ExpiresAt <= _clock().ToUniversalTime())
        {
            return "invalid_fence";
        }
        if (_fence is not null && fence == _fence)
        {
            if (_release is null || request.Release != _release)
            {
                return "release_mismatch";
            }
            return string.Empty;
        }
        if (_release is not null && request.Release != _release)
        {
            return "release_mismatch";
        }
        if (_fence is not null)
        {
            if (fence.AuthorityGeneration <= _fence.AuthorityGeneration
                || fence.FenceGeneration <= _fence.FenceGeneration)
            {
                return "stale_boot";
            }
        }
        return string.Empty;
    }

    private bool CurrentLeaseMatchesLocked(RuntimeV3OperationKey operation) =>
        _fence is not null
        && string.Equals(_fence.InstanceId, operation.InstanceId, StringComparison.Ordinal)
        && string.Equals(_fence.LeaseId, operation.LeaseId, StringComparison.Ordinal)
        && _fence.LeaseEpoch == operation.LeaseEpoch;

    private bool TicketMatchesFenceLocked(RuntimeV3HostAdmissionTicket ticket) =>
        _fence is not null
        && ticket.BootId == _fence.BootId
        && ticket.InstanceIncarnation == _fence.InstanceIncarnation
        && ticket.LeaseEpoch == _fence.LeaseEpoch
        && ticket.HostFenceId == _fence.HostFenceId
        && ticket.AuthorityGeneration == _fence.AuthorityGeneration
        && ticket.FenceGeneration == _fence.FenceGeneration
        && CurrentLeaseMatchesLocked(ticket.Operation);

    private static bool ValidFence(RuntimeV3HostFence fence) =>
        RuntimeV3GameplayRecoveryContract.IsUuid(fence.DeploymentId)
        && RuntimeV3GameplayRecoveryContract.IsUuid(fence.InstanceId)
        && RuntimeV3GameplayRecoveryContract.IsRandomUuid(fence.InstanceIncarnation)
        && RuntimeV3GameplayRecoveryContract.IsRandomUuid(fence.BootId)
        && fence.AuthorityGeneration is > 0 and <= RuntimeV3GameplayRecoveryContract.MaxWireInteger
        && RuntimeV3GameplayRecoveryContract.IsRandomUuid(fence.LeaseId)
        && fence.LeaseEpoch is > 0 and <= RuntimeV3GameplayRecoveryContract.MaxWireInteger
        && RuntimeV3GameplayRecoveryContract.IsRandomUuid(fence.HostFenceId)
        && fence.FenceGeneration is > 0 and <= RuntimeV3GameplayRecoveryContract.MaxWireInteger
        && RuntimeV3GameplayRecoveryContract.IsTimestamp(fence.ExpiresAt);

    private static bool ValidRelease(RuntimeV3RecoveryRelease release) =>
        RuntimeV3GameplayRecoveryContract.IsDigest(release.ReleaseDigest)
        && RuntimeV3GameplayRecoveryContract.IsDigest(release.ConfigDigest)
        && RuntimeV3GameplayRecoveryContract.IsDigest(release.ProfileDigest)
        && release.RuntimeV3SchemaDigest == RuntimeV3GameplayContract.SchemaDigest;

    private static string FenceBytes(RuntimeV3HostFence fence, RuntimeV3RecoveryRelease release) =>
        string.Join("|", RuntimeV3GameplayRecoveryContract.Contract,
            RuntimeV3GameplayRecoveryContract.SchemaDigest, RuntimeV3GameplayContract.SchemaDigest,
            fence.DeploymentId, fence.InstanceId, fence.InstanceIncarnation, fence.BootId,
            fence.AuthorityGeneration, fence.LeaseId, fence.LeaseEpoch, fence.HostFenceId,
            fence.FenceGeneration, fence.ExpiresAt.UtcDateTime.ToString("O"),
            release.ReleaseDigest, release.ConfigDigest, release.ProfileDigest,
            release.RuntimeV3SchemaDigest);

    private static string SerializeFence(
        RuntimeV3HostFence fence, RuntimeV3RecoveryRelease release) =>
        JsonSerializer.Serialize(new FencePayload(fence, release));

    private bool AppendOperationLocked(RuntimeV3HostOperationRecord record) =>
        AppendEventLocked("operation", JsonSerializer.Serialize(new OperationPayload(record)));

    private bool AppendEventLocked(string kind, string payload)
    {
        if (!_healthy || _disposed || _journal.Length > RuntimeV3GameplayRecoveryContract.MaxJournalBytes)
        {
            _healthy = false;
            return false;
        }

        try
        {
            long sequence = ++_sequence;
            string digest = RuntimeV3GameplayRecoveryContract.Digest(
                Encoding.UTF8.GetBytes($"{sequence}|{kind}|{payload}"));
            string line = JsonSerializer.Serialize(new JournalPayload(sequence, kind, payload, digest))
                + Environment.NewLine;
            byte[] bytes = Encoding.UTF8.GetBytes(line);
            if (_journal.Length + bytes.Length > RuntimeV3GameplayRecoveryContract.MaxJournalBytes)
            {
                _healthy = false;
                return false;
            }
            _journal.Seek(0, SeekOrigin.End);
            _journal.Write(bytes, 0, bytes.Length);
            _journal.Flush(true);
            return true;
        }
        catch (IOException)
        {
            _healthy = false;
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            _healthy = false;
            return false;
        }
    }

    private void ReplayLocked()
    {
        _journal.Seek(0, SeekOrigin.Begin);
        using var reader = new StreamReader(_journal, new UTF8Encoding(false, true), false, 64 * 1024, true);
        string? line;
        long expectedSequence = 1;
        while ((line = reader.ReadLine()) is not null)
        {
            if (line.Length > RuntimeV3GameplayRecoveryContract.MaxFrameBytes)
            {
                throw new InvalidDataException("recovery journal frame exceeds its bound");
            }
            using JsonDocument document = JsonDocument.Parse(line);
            if (!Exact(document.RootElement, "Sequence", "Kind", "Payload", "Digest"))
            {
                throw new InvalidDataException("recovery journal frame has an unknown field");
            }
            JsonElement root = document.RootElement;
            if (!root.TryGetProperty("Sequence", out JsonElement sequenceValue)
                || !sequenceValue.TryGetInt64(out long sequence)
                || sequence != expectedSequence
                || !Text(root, "Kind", out string? kind)
                || !Text(root, "Payload", out string? payload)
                || !Text(root, "Digest", out string? digest)
                || !RuntimeV3GameplayRecoveryContract.IsDigest(digest!)
                || !string.Equals(digest, RuntimeV3GameplayRecoveryContract.Digest(
                    Encoding.UTF8.GetBytes($"{sequence}|{kind}|{payload}")), StringComparison.Ordinal))
            {
                throw new InvalidDataException("recovery journal integrity or sequence check failed");
            }
            ApplyEventLocked(kind!, payload!);
            _sequence = sequence;
            expectedSequence++;
        }
    }

    private void ApplyEventLocked(string kind, string payload)
    {
        switch (kind)
        {
            case "store_initialized":
                if (payload != "{}") throw new InvalidDataException("invalid store event");
                return;
            case "fence_replaced":
                FencePayload? fence = JsonSerializer.Deserialize<FencePayload>(payload, StrictJournalJson);
                if (fence is null || !ValidFence(fence.ToFence()) || !ValidRelease(fence.ToRelease()))
                    throw new InvalidDataException("invalid persisted fence");
                _fence = fence.ToFence();
                _release = fence.ToRelease();
                return;
            case "operation":
                OperationPayload? operation = JsonSerializer.Deserialize<OperationPayload>(payload, StrictJournalJson);
                if (operation is null || !operation.TryRestore(out RuntimeV3HostOperationRecord? restored))
                    throw new InvalidDataException("invalid persisted operation");
                _operations[restored!.Operation] = restored;
                if (_operations.Count > RuntimeV3GameplayRecoveryContract.MaxOperationRecords)
                    throw new InvalidDataException("persisted operation capacity exceeded");
                return;
            default:
                throw new InvalidDataException("unknown recovery event");
        }
    }

    private static bool Exact(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object)
        {
            return false;
        }
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            if (!names.Add(property.Name))
            {
                return false;
            }
        }
        if (names.Count != fields.Length)
        {
            return false;
        }
        foreach (string field in fields)
        {
            if (!names.Contains(field))
            {
                return false;
            }
        }
        return true;
    }

    private static bool Text(JsonElement root, string name, out string? value)
    {
        value = null;
        return root.TryGetProperty(name, out JsonElement property)
            && property.ValueKind == JsonValueKind.String
            && (value = property.GetString()) is not null;
    }

    private sealed record JournalPayload(long Sequence, string Kind, string Payload, string Digest);

}
