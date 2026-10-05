// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal readonly record struct ProgressReadinessSnapshot(
    Guid RuntimeIncarnation,
    long Generation,
    long LoadAttempt,
    int ProfileId,
    int ReadSaveStatus,
    int OwnerThreadId);

internal readonly record struct ProgressReadinessLoadAttempt(
    Guid RuntimeIncarnation,
    long Generation,
    long AttemptId,
    int OwnerThreadId,
    object Instance,
    int? ProfileId,
    bool CanInspectHost);

internal readonly record struct ProgressReadinessLease(
    Guid RuntimeIncarnation,
    long Generation,
    object Instance,
    object Progress,
    int ProfileId);

internal sealed partial class ProgressReadinessState
{
    private const int MaximumActiveProgressLoads = 64;
    private readonly object _gate = new();
    private readonly HashSet<long> _activeAttemptIds = new();
    private readonly Guid _runtimeIncarnation = Guid.NewGuid();
    private long _generation;
    private long _nextAttemptId;
    private long _attemptId;
    private int _ownerThreadId;
    private bool _observerAttached;
    private bool _patchesVerified;
    private bool _disabled;
    private bool _ready;
    private bool _nestedLoadObserved;
    private object? _instance;
    private object? _progress;
    private int _profileId;
    private int _readSaveStatus;

    internal Guid RuntimeIncarnation => _runtimeIncarnation;

    internal bool IsOperational
    {
        get
        {
            lock (_gate)
            {
                return !_disabled;
            }
        }
    }

    internal void MarkObserverAttached()
    {
        lock (_gate)
        {
            if (!_disabled)
            {
                _observerAttached = true;
            }
        }
    }

    internal bool ObserveProcessFrame(int threadId)
    {
        lock (_gate)
        {
            if (_disabled || !_observerAttached || threadId <= 0)
            {
                DisableLocked();
                return false;
            }

            if (_ownerThreadId == 0)
            {
                _ownerThreadId = threadId;
                return true;
            }

            if (_ownerThreadId == threadId)
            {
                return true;
            }

            DisableLocked();
            return false;
        }
    }

    internal void MarkPatchesVerified()
    {
        lock (_gate)
        {
            if (!_disabled)
            {
                _patchesVerified = true;
            }
        }
    }

    internal void Disable()
    {
        lock (_gate)
        {
            DisableLocked();
        }
    }

    internal void Invalidate()
    {
        lock (_gate)
        {
            InvalidateLocked();
        }
    }

    internal void BeforeProfileInitialization() => Invalidate();

    internal void BeforeProfileSwitch() => Invalidate();

    internal void BeforeProfileDeletion() => Invalidate();

    internal void BeforeProgressReplacement() => Invalidate();

    internal ProgressReadinessLoadAttempt BeginProgressLoad(object instance, int threadId)
    {
        lock (_gate)
        {
            if (_disabled)
            {
                return default;
            }

            if (_activeAttemptIds.Count > 0)
            {
                _nestedLoadObserved = true;
            }

            InvalidateLocked();
            if (_disabled || _activeAttemptIds.Count >= MaximumActiveProgressLoads)
            {
                DisableLocked();
                return default;
            }

            long attemptId = NextAttemptLocked();
            if (attemptId == 0)
            {
                return default;
            }

            _activeAttemptIds.Add(attemptId);
            bool canInspect = _activeAttemptIds.Count == 1
                && !_nestedLoadObserved
                && IsOwnerThreadLocked(threadId)
                && _patchesVerified
                && attemptId != 0;
            return new ProgressReadinessLoadAttempt(
                _runtimeIncarnation,
                _generation,
                attemptId,
                threadId,
                instance,
                null,
                canInspect);
        }
    }

    internal ProgressReadinessLoadAttempt BindProfile(
        ProgressReadinessLoadAttempt attempt,
        bool initialized,
        int? profileId)
    {
        lock (_gate)
        {
            bool valid = attempt.CanInspectHost
                && IsCurrentAttemptLocked(attempt)
                && initialized
                && profileId.HasValue;
            return attempt with { ProfileId = profileId, CanInspectHost = valid };
        }
    }

    internal bool CanInspectAttempt(ProgressReadinessLoadAttempt attempt, int threadId)
    {
        lock (_gate)
        {
            return attempt.CanInspectHost
                && attempt.OwnerThreadId == threadId
                && IsCurrentAttemptLocked(attempt);
        }
    }

    private void InvalidateLocked()
    {
        _ready = false;
        _instance = null;
        _progress = null;
        _profileId = 0;
        _readSaveStatus = -1;
        _attemptId = 0;
        if (_generation == long.MaxValue)
        {
            DisableLocked();
            return;
        }

        _generation++;
    }

    private long NextAttemptLocked()
    {
        if (_disabled || _generation == long.MaxValue || _nextAttemptId == long.MaxValue)
        {
            DisableLocked();
            return 0;
        }

        _nextAttemptId++;
        _attemptId = _nextAttemptId;
        return _attemptId;
    }

    private void DisableLocked()
    {
        _disabled = true;
        _ready = false;
        _nestedLoadObserved = false;
        _activeAttemptIds.Clear();
        _instance = null;
        _progress = null;
        _profileId = 0;
        _readSaveStatus = -1;
        _attemptId = 0;
        if (_generation < long.MaxValue)
        {
            _generation++;
        }
    }

}
