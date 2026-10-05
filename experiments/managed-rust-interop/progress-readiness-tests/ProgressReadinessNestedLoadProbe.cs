// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessNestedLoadProbe
{
    internal static int Run()
    {
        ProgressReadinessState state = new();
        state.MarkObserverAttached();
        Assert(state.ObserveProcessFrame(17), "frame establishes owner thread");
        state.MarkPatchesVerified();
        object manager = new();
        ProgressReadinessLoadAttempt outer = Begin(state, manager);
        ProgressReadinessLoadAttempt inner = Begin(state, manager);
        object progress = new();
        int innerResultReads = 0;
        Assert(!Complete(state, inner, manager, progress, () => { innerResultReads++; return (true, 0, true); }), "inner attempt refused");
        Assert(innerResultReads == 0, "inner attempt does not inspect result");
        ProgressReadinessCallbacks.FinalizeProgressLoad(state, null, inner);
        Assert(!state.TryGetLease(17, out _), "outer remains active during inner unwind");
        int outerResultReads = 0;
        Assert(!Complete(state, outer, manager, progress, () => { outerResultReads++; return (true, 0, true); }), "stale outer attempt refused");
        Assert(outerResultReads == 0, "stale outer attempt does not inspect result");
        ProgressReadinessCallbacks.FinalizeProgressLoad(state, null, outer);
        Assert(!state.TryGetLease(17, out _), "drained nested chain remains unavailable");
        ProgressReadinessLoadAttempt fresh = Begin(state, manager);
        Assert(Complete(state, fresh, manager, progress, () => (true, 0, true)), "fresh nonnested load accepted");
        ProgressReadinessCallbacks.FinalizeProgressLoad(state, null, fresh);
        Assert(state.TryGetLease(17, out _), "fresh load restores readiness after finalizer");
        return 9;
    }

    private static ProgressReadinessLoadAttempt Begin(ProgressReadinessState state, object manager) =>
        ProgressReadinessCallbacks.BeginProgressLoad(
            state,
            manager,
            17,
            () => new ProgressReadinessHostIdentity(true, 1, null));

    private static bool Complete(
        ProgressReadinessState state,
        ProgressReadinessLoadAttempt attempt,
        object manager,
        object progress,
        Func<(bool Success, int Status, bool SavedDataPresent)> readResult) =>
        ProgressReadinessCallbacks.CompleteProgressLoad(
            state,
            attempt,
            manager,
            17,
            readResult,
            () => new ProgressReadinessHostIdentity(true, 1, progress));

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException($"failed: {message}");
        }
    }
}
