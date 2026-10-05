// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class ProgressReadinessState
{
    internal bool TryPublish(
        ProgressReadinessLoadAttempt attempt,
        object instance,
        bool initializedBefore,
        int? profileBefore,
        object? progressBefore,
        bool initializedAfter,
        int? profileAfter,
        object? progressAfter,
        bool success,
        int status,
        bool savedDataPresent,
        int threadId)
    {
        lock (_gate)
        {
            if (!attempt.CanInspectHost || !IsCurrentAttemptLocked(attempt))
            {
                return false;
            }

            if (attempt.OwnerThreadId != threadId
                || !IsOwnerThreadLocked(threadId)
                || !ReferenceEquals(attempt.Instance, instance)
                || !initializedBefore
                || !initializedAfter
                || !attempt.ProfileId.HasValue
                || profileBefore != attempt.ProfileId
                || profileAfter != attempt.ProfileId
                || progressBefore == null
                || !ReferenceEquals(progressBefore, progressAfter)
                || !IsAcceptedLoadResult(success, status, savedDataPresent))
            {
                InvalidateLocked();
                return false;
            }

            _instance = instance;
            _progress = progressBefore;
            _profileId = attempt.ProfileId.Value;
            _readSaveStatus = status;
            _ready = true;
            return true;
        }
    }

    internal void FailAttempt(ProgressReadinessLoadAttempt attempt)
    {
        lock (_gate)
        {
            if (IsCurrentAttemptLocked(attempt))
            {
                InvalidateLocked();
            }
        }
    }

    internal void FailLease(ProgressReadinessLease lease)
    {
        lock (_gate)
        {
            if (IsCurrentLeaseLocked(lease))
            {
                InvalidateLocked();
            }
        }
    }

    internal Exception? FinalizeProgressLoad(
        Exception? originalException,
        ProgressReadinessLoadAttempt attempt)
    {
        lock (_gate)
        {
            if (attempt.RuntimeIncarnation != _runtimeIncarnation
                || attempt.AttemptId == 0
                || !_activeAttemptIds.Remove(attempt.AttemptId))
            {
                return originalException;
            }

            if (originalException != null && IsCurrentAttemptLocked(attempt))
            {
                InvalidateLocked();
            }

            if (_activeAttemptIds.Count == 0 && _nestedLoadObserved)
            {
                _nestedLoadObserved = false;
                InvalidateLocked();
            }
        }

        return originalException;
    }

    internal bool TryGetLease(int threadId, out ProgressReadinessLease lease)
    {
        lock (_gate)
        {
            if (!_ready || !IsOwnerThreadLocked(threadId) || _instance == null || _progress == null)
            {
                lease = default;
                return false;
            }

            lease = new ProgressReadinessLease(
                _runtimeIncarnation,
                _generation,
                _instance,
                _progress,
                _profileId);
            return true;
        }
    }

    internal bool TryCopyValidatedLease(
        ProgressReadinessLease lease,
        int threadId,
        bool hostStateMatches,
        out ProgressReadinessSnapshot snapshot)
    {
        lock (_gate)
        {
            if (!IsCurrentLeaseLocked(lease) || !IsOwnerThreadLocked(threadId))
            {
                snapshot = default;
                return false;
            }

            if (!hostStateMatches)
            {
                InvalidateLocked();
                snapshot = default;
                return false;
            }

            snapshot = new ProgressReadinessSnapshot(
                _runtimeIncarnation,
                _generation,
                _attemptId,
                _profileId,
                _readSaveStatus,
                _ownerThreadId);
            return true;
        }
    }

    private bool IsOwnerThreadLocked(int threadId) =>
        !_disabled && _observerAttached && _ownerThreadId > 0 && threadId == _ownerThreadId;

    private bool IsCurrentAttemptLocked(ProgressReadinessLoadAttempt attempt) =>
        !_disabled
        && attempt.RuntimeIncarnation == _runtimeIncarnation
        && attempt.Generation == _generation
        && attempt.AttemptId != 0
        && attempt.AttemptId == _attemptId;

    private bool IsCurrentLeaseLocked(ProgressReadinessLease lease) =>
        !_disabled
        && _ready
        && lease.RuntimeIncarnation == _runtimeIncarnation
        && lease.Generation == _generation
        && ReferenceEquals(lease.Instance, _instance)
        && ReferenceEquals(lease.Progress, _progress)
        && lease.ProfileId == _profileId;

    private static bool IsAcceptedLoadResult(bool success, int status, bool savedDataPresent) =>
        (success && savedDataPresent && status is 0 or 8 or 9 or 10)
        || (!success && !savedDataPresent && status == 2);
}
