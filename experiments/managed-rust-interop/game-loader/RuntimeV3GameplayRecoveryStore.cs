// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Concrete owner-local journal for host fences, admission tickets, and operation receipts.
/// Every appended transition is flushed through the file handle before the caller may invoke
/// host code. A corrupt or unavailable journal remains blocked; it is never recreated silently.
/// </summary>
internal sealed partial class RuntimeV3GameplayRecoveryStore : IDisposable
{
    private const string JournalFileName = "runtime-v3-recovery-v1.journal";
    private const string LockFileName = "runtime-v3-recovery-v1.lock";
    private static readonly JsonSerializerOptions StrictJournalJson = new()
    {
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow
    };
    private readonly object _gate = new();
    private readonly object _executionGate = new();
    private readonly Dictionary<RuntimeV3OperationKey, RuntimeV3HostOperationRecord> _operations = new();
    private readonly RuntimeV3RecoveryCredentials _credentials;
    private readonly Func<DateTimeOffset> _clock;
    private readonly FileStream _lockStream;
    private readonly FileStream _journal;
    private RuntimeV3HostFence? _fence;
    private RuntimeV3RecoveryRelease? _release;
    private long _sequence;
    private bool _healthy = true;
    private bool _disposed;

    internal RuntimeV3GameplayRecoveryStore(
        string directory,
        RuntimeV3RecoveryCredentials credentials,
        Func<DateTimeOffset>? clock = null)
    {
        if (string.IsNullOrWhiteSpace(directory) || !Path.IsPathFullyQualified(directory))
        {
            throw new ArgumentException("recovery storage must be an absolute owner-local path", nameof(directory));
        }

        DirectoryInfo root = Directory.CreateDirectory(directory);
        if (root.Attributes.HasFlag(FileAttributes.ReparsePoint))
        {
            throw new IOException("recovery storage cannot be a reparse-point directory");
        }

        _credentials = credentials;
        _clock = clock ?? (() => DateTimeOffset.UtcNow);
        _lockStream = new FileStream(Path.Combine(directory, LockFileName), FileMode.OpenOrCreate,
            FileAccess.ReadWrite, FileShare.None, 4096, FileOptions.WriteThrough);
        _journal = new FileStream(Path.Combine(directory, JournalFileName), FileMode.OpenOrCreate,
            FileAccess.ReadWrite, FileShare.Read, 64 * 1024, FileOptions.WriteThrough);
        try
        {
            if (_journal.Length == 0)
            {
                AppendEventLocked("store_initialized", "{}");
            }
            else
            {
                ReplayLocked();
            }
        }
        catch
        {
            _healthy = false;
        }
    }

    internal static bool TryOpen(
        string directory,
        RuntimeV3RecoveryCredentials credentials,
        out RuntimeV3GameplayRecoveryStore? store,
        out string error,
        Func<DateTimeOffset>? clock = null)
    {
        store = null;
        try
        {
            store = new RuntimeV3GameplayRecoveryStore(directory, credentials, clock);
            if (!store.IsHealthy)
            {
                error = "persistence_unavailable";
                store.Dispose();
                store = null;
                return false;
            }
            error = string.Empty;
            return true;
        }
        catch (IOException)
        {
            error = "persistence_busy_or_unavailable";
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            error = "persistence_unauthorized";
            return false;
        }
        catch (ArgumentException)
        {
            error = "invalid_recovery_storage";
            return false;
        }
    }

    internal bool IsHealthy
    {
        get { lock (_gate) { return _healthy && !_disposed; } }
    }

    internal RuntimeV3HostFence? CurrentFence
    {
        get { lock (_gate) { return _fence; } }
    }

    internal int OperationCount
    {
        get { lock (_gate) { return _operations.Count; } }
    }

    internal bool HasPendingMutation
    {
        get
        {
            lock (_gate)
            {
                foreach (RuntimeV3HostOperationRecord record in _operations.Values)
                {
                    if (record.Status is RuntimeV3DispatchStatus.Accepted
                        or RuntimeV3DispatchStatus.Unknown)
                    {
                        return true;
                    }
                }
                return false;
            }
        }
    }

    /// <summary>
    /// Serializes the host execution window with fence replacement. The caller must hold the
    /// returned lease from the final ticket check through the host callback and completion probe;
    /// otherwise a replacement controller could linearize between those two steps.
    /// </summary>
    internal IDisposable AcquireExecutionFence()
    {
        Monitor.Enter(_executionGate);
        return new ExecutionFenceLease(_executionGate);
    }

    private sealed class ExecutionFenceLease : IDisposable
    {
        private readonly object _gate;
        private bool _released;

        internal ExecutionFenceLease(object gate) { _gate = gate; }

        public void Dispose()
        {
            if (_released)
            {
                return;
            }
            _released = true;
            Monitor.Exit(_gate);
        }
    }

    internal bool TryMarkRejected(
        RuntimeV3OperationKey operation, string errorCode, out string error)
    {
        return Transition(operation, RuntimeV3HostOperationState.Rejected,
            RuntimeV3DispatchStatus.Rejected, false, errorCode,
            RuntimeV3HostTicketState.Rejected, null, null, out error);
    }

    internal bool TryMarkUnknown(
        RuntimeV3OperationKey operation, string errorCode, out string error)
    {
        return Transition(operation, RuntimeV3HostOperationState.Unknown,
            RuntimeV3DispatchStatus.Unknown, true, errorCode,
            RuntimeV3HostTicketState.Unknown, null, null, out error);
    }

    internal bool TryMarkSettled(
        RuntimeV3OperationKey operation,
        RuntimeV3TransitionWitness witness,
        out string error)
    {
        return Transition(operation, RuntimeV3HostOperationState.Settled,
            RuntimeV3DispatchStatus.Settled, true, null,
            RuntimeV3HostTicketState.Settled, witness, null, out error);
    }

    internal bool TryGet(
        RuntimeV3OperationKey operation, out RuntimeV3HostOperationRecord? record)
    {
        lock (_gate)
        {
            return _operations.TryGetValue(operation, out record);
        }
    }

    internal bool TryHistoricalLookup(
        RuntimeV3OperationKey operation,
        string payloadDigest,
        string proof,
        out RuntimeV3HistoricalOperation? result,
        out string error)
    {
        lock (_gate)
        {
            result = null;
            error = string.Empty;
            if (!_healthy || _disposed)
            {
                error = "persistence_unavailable";
                return false;
            }
            if (!RuntimeV3GameplayRecoveryContract.IsDigest(payloadDigest)
                || !RuntimeV3GameplayRecoveryContract.VerifyProof(
                    _credentials.HistoricalReadSecret,
                    "historical-read",
                    string.Join("|", operation.InstanceId, operation.SessionId, operation.LeaseId,
                        operation.LeaseEpoch, operation.OperationId, payloadDigest),
                    proof))
            {
                error = "auth_required";
                return false;
            }
            if (!_operations.TryGetValue(operation, out RuntimeV3HostOperationRecord? record)
                || record.PayloadDigest != payloadDigest)
            {
                error = "operation_not_found";
                return false;
            }

            result = new RuntimeV3HistoricalOperation(record, false);
            return true;
        }
    }

    public void Dispose()
    {
        lock (_gate)
        {
            if (_disposed)
            {
                return;
            }
            _disposed = true;
            _journal.Dispose();
            _lockStream.Dispose();
        }
    }
}
