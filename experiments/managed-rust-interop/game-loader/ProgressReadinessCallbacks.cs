// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal readonly record struct ProgressReadinessHostIdentity(
    bool Initialized,
    int? ProfileId,
    object? Progress);

internal static class ProgressReadinessCallbacks
{
    internal static ProgressReadinessLoadAttempt BeginProgressLoad(
        ProgressReadinessState state,
        object instance,
        int threadId,
        Func<ProgressReadinessHostIdentity> readIdentity)
    {
        ProgressReadinessLoadAttempt attempt = default;
        try
        {
            attempt = state.BeginProgressLoad(instance, threadId);
            if (!state.CanInspectAttempt(attempt, threadId))
            {
                return attempt;
            }

            ProgressReadinessHostIdentity current = readIdentity();
            return state.BindProfile(attempt, current.Initialized, current.ProfileId);
        }
        catch (Exception)
        {
            if (attempt.RuntimeIncarnation == Guid.Empty)
            {
                DisableSafely(state);
            }
            else
            {
                FailAttempt(state, attempt);
            }

            return attempt with { CanInspectHost = false };
        }
    }

    internal static bool CompleteProgressLoad(
        ProgressReadinessState state,
        ProgressReadinessLoadAttempt attempt,
        object instance,
        int threadId,
        Func<(bool Success, int Status, bool SavedDataPresent)> readResult,
        Func<ProgressReadinessHostIdentity> readIdentity)
    {
        try
        {
            if (!state.CanInspectAttempt(attempt, threadId))
            {
                return false;
            }

            (bool success, int status, bool savedDataPresent) = readResult();
            ProgressReadinessHostIdentity first = readIdentity();
            ProgressReadinessHostIdentity second = readIdentity();
            return state.TryPublish(
                attempt,
                instance,
                first.Initialized,
                first.ProfileId,
                first.Progress,
                second.Initialized,
                second.ProfileId,
                second.Progress,
                success,
                status,
                savedDataPresent,
                threadId);
        }
        catch (Exception)
        {
            FailAttempt(state, attempt);
            return false;
        }
    }

    internal static bool TryCaptureOwnedSnapshot(
        ProgressReadinessState state,
        int threadId,
        Func<object, ProgressReadinessHostIdentity> readIdentity,
        out ProgressReadinessSnapshot snapshot)
    {
        snapshot = default;
        ProgressReadinessLease lease = default;
        try
        {
            if (!state.TryGetLease(threadId, out lease))
            {
                return false;
            }

            ProgressReadinessHostIdentity first = readIdentity(lease.Instance);
            ProgressReadinessHostIdentity second = readIdentity(lease.Instance);
            bool matches = first.Initialized
                && second.Initialized
                && first.ProfileId == lease.ProfileId
                && second.ProfileId == lease.ProfileId
                && first.Progress != null
                && ReferenceEquals(first.Progress, second.Progress)
                && ReferenceEquals(first.Progress, lease.Progress);
            return state.TryCopyValidatedLease(lease, threadId, matches, out snapshot);
        }
        catch (Exception)
        {
            if (lease.RuntimeIncarnation == Guid.Empty)
            {
                DisableSafely(state);
            }
            else
            {
                FailLease(state, lease);
            }

            return false;
        }
    }

    internal static Exception? FinalizeProgressLoad(
        ProgressReadinessState state,
        Exception? originalException,
        ProgressReadinessLoadAttempt attempt)
    {
        try
        {
            return state.FinalizeProgressLoad(originalException, attempt);
        }
        catch (Exception)
        {
            DisableSafely(state);
            return originalException;
        }
    }

    internal static void AbortProgressLoad(ProgressReadinessState state, ProgressReadinessLoadAttempt attempt) =>
        FailAttempt(state, attempt);

    private static void FailAttempt(ProgressReadinessState state, ProgressReadinessLoadAttempt attempt)
    {
        try
        {
            state.FailAttempt(attempt);
        }
        catch (Exception)
        {
            DisableSafely(state);
        }
    }

    private static void FailLease(ProgressReadinessState state, ProgressReadinessLease lease)
    {
        try
        {
            state.FailLease(lease);
        }
        catch (Exception)
        {
            DisableSafely(state);
        }
    }

    private static void DisableSafely(ProgressReadinessState state)
        => ProgressReadinessRuntime.DisableSafely(state);
}
