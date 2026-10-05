// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessProbe
{
    private static int _checks;

    private static int Main()
    {
        VerifyAcceptedStatusesAndQuality();
        VerifyRejectedResultsAndIdentityChanges();
        VerifyInvalidationAndStaleCompletion();
        _checks += ProgressReadinessNestedLoadProbe.Run();
        _checks += ProgressReadinessHostPinProbe.Run();
        VerifyOwnerThreadAndRuntimeIncarnation();
        VerifyRegistrationAndFinalizer();
        _checks += ProgressReadinessPatchBindingProbe.Run();
        Console.WriteLine($"progress_readiness_checks={_checks}");
        return 0;
    }

    private static void VerifyAcceptedStatusesAndQuality()
    {
        foreach (int status in new[] { 0, 8, 9, 10 })
        {
            ProgressReadinessState state = NewReadyState();
            object manager = new();
            object progress = new();
            ProgressReadinessLoadAttempt attempt = Begin(state, manager, 2);
            True(Publish(state, attempt, manager, 2, progress, progress, true, status, true), $"status {status} accepted");
            ProgressReadinessSnapshot snapshot = Read(state, manager, progress, 2);
            Equal(status, snapshot.ReadSaveStatus, $"status {status} retained");
            Equal(2, snapshot.ProfileId, $"status {status} profile retained");
            Equal(state.RuntimeIncarnation, snapshot.RuntimeIncarnation, $"status {status} incarnation retained");
        }

        ProgressReadinessState fresh = NewReadyState();
        object freshManager = new();
        object defaultProgress = new();
        ProgressReadinessLoadAttempt freshAttempt = Begin(fresh, freshManager, 1);
        True(Publish(fresh, freshAttempt, freshManager, 1, defaultProgress, defaultProgress, false, 2, false), "missing save accepts in-memory default");
        Equal(2, Read(fresh, freshManager, defaultProgress, 1).ReadSaveStatus, "fresh profile quality retained");
    }

    private static void VerifyRejectedResultsAndIdentityChanges()
    {
        foreach (int status in new[] { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 42 })
        {
            foreach (bool success in new[] { false, true })
            {
                foreach (bool savedData in new[] { false, true })
                {
                    bool expected = (success && savedData && status is 0 or 8 or 9 or 10)
                        || (!success && !savedData && status == 2);
                    AssertResult(success, status, savedData, expected, $"status {status}, success={success}, data={savedData}");
                }
            }
        }

        ProgressReadinessState wrongInstance = NewReadyState();
        object original = new();
        ProgressReadinessLoadAttempt attempt = Begin(wrongInstance, original, 1);
        object stableProgress = new();
        False(Publish(wrongInstance, attempt, new object(), 1, stableProgress, stableProgress, true, 0, true), "other manager instance refused");

        ProgressReadinessState changedProfile = NewReadyState();
        object manager = new();
        ProgressReadinessLoadAttempt profileAttempt = Begin(changedProfile, manager, 1);
        object profileProgress = new();
        False(Publish(changedProfile, profileAttempt, manager, 2, profileProgress, profileProgress, true, 0, true), "changed profile refused");

        ProgressReadinessState changedProgress = NewReadyState();
        object progressManager = new();
        ProgressReadinessLoadAttempt progressAttempt = Begin(changedProgress, progressManager, 1);
        False(Publish(changedProgress, progressAttempt, progressManager, 1, new object(), new object(), true, 0, true), "replaced progress object refused");

        ProgressReadinessState captureState = NewReadyState();
        object captureManager = new();
        object captureProgress = new();
        ProgressReadinessLoadAttempt captureAttempt = Begin(captureState, captureManager, 1);
        True(Publish(captureState, captureAttempt, captureManager, 1, captureProgress, captureProgress, true, 0, true), "capture precondition ready");
        False(TryCapture(captureState, captureManager, captureProgress, 2), "current profile rechecked before copying");
        False(captureState.TryGetLease(17, out _), "profile mismatch revokes retained lease");

        ProgressReadinessState movedProgress = NewReadyState();
        object movedManager = new();
        object initialProgress = new();
        ProgressReadinessLoadAttempt movedAttempt = Begin(movedProgress, movedManager, 1);
        True(Publish(movedProgress, movedAttempt, movedManager, 1, initialProgress, initialProgress, true, 0, true), "progress capture precondition ready");
        False(TryCapture(movedProgress, movedManager, new object(), 1), "current progress object rechecked before copying");
    }

    private static void VerifyInvalidationAndStaleCompletion()
    {
        Action<ProgressReadinessState>[] invalidations =
        {
            state => state.BeforeProfileInitialization(),
            state => state.BeforeProfileSwitch(),
            state => state.BeforeProfileDeletion(),
            state => state.BeforeProgressReplacement(),
            state => state.Invalidate()
        };

        foreach (Action<ProgressReadinessState> invalidate in invalidations)
        {
            ProgressReadinessState state = NewReadyState();
            object manager = new();
            ProgressReadinessLoadAttempt attempt = Begin(state, manager, 1);
            object progress = new();
            True(Publish(state, attempt, manager, 1, progress, progress, true, 0, true), "precondition ready");
            invalidate(state);
            False(state.TryGetLease(17, out _), "lifecycle prefix revokes before host mutation");
        }

        ProgressReadinessState stale = NewReadyState();
        object staleManager = new();
        ProgressReadinessLoadAttempt oldAttempt = Begin(stale, staleManager, 1);
        stale.BeforeProfileSwitch();
        ProgressReadinessCallbacks.FinalizeProgressLoad(stale, null, oldAttempt);
        object newManager = new();
        ProgressReadinessLoadAttempt newAttempt = Begin(stale, newManager, 3);
        False(Publish(stale, oldAttempt, staleManager, 1, new object(), new object(), true, 0, true), "old load cannot restore readiness");
        object currentProgress = new();
        True(Publish(stale, newAttempt, newManager, 3, currentProgress, currentProgress, true, 9, true), "new generation can become ready");

    }

    private static void VerifyOwnerThreadAndRuntimeIncarnation()
    {
        ProgressReadinessState unknownOwner = new();
        unknownOwner.MarkObserverAttached();
        unknownOwner.MarkPatchesVerified();
        object manager = new();
        int getterCalls = 0;
        ProgressReadinessLoadAttempt attempt = ProgressReadinessCallbacks.BeginProgressLoad(
            unknownOwner,
            manager,
            17,
            () =>
            {
                getterCalls++;
                return new ProgressReadinessHostIdentity(true, 1, null);
            });
        object progress = new();
        False(Publish(unknownOwner, attempt, manager, 1, progress, progress, true, 0, true), "subscription alone does not establish owner thread");
        Equal(0, getterCalls, "unknown thread does not read profile or progress getters");
        False(unknownOwner.TryGetLease(17, out _), "unknown owner remains unavailable");

        ProgressReadinessState wrongThread = NewReadyState();
        False(wrongThread.ObserveProcessFrame(18), "owner thread change disables readiness");
        False(wrongThread.TryGetLease(17, out _), "changed owner thread is unavailable");

        ProgressReadinessState wrongThreadLoad = NewReadyState();
        object wrongThreadManager = new();
        ProgressReadinessLoadAttempt wrongThreadAttempt = Begin(wrongThreadLoad, wrongThreadManager, 1);
        int wrongThreadResultReads = 0;
        int wrongThreadIdentityReads = 0;
        False(CompleteWithReaders(
            wrongThreadLoad,
            wrongThreadAttempt,
            wrongThreadManager,
            18,
            () => { wrongThreadResultReads++; return (true, 0, true); },
            () => { wrongThreadIdentityReads++; return new ProgressReadinessHostIdentity(true, 1, new object()); }), "wrong-thread completion refused");
        Equal(0, wrongThreadResultReads, "wrong-thread completion does not inspect host result");
        Equal(0, wrongThreadIdentityReads, "wrong-thread completion does not inspect host identity");

        ProgressReadinessState oldRuntime = NewReadyState();
        object oldManager = new();
        ProgressReadinessLoadAttempt oldAttempt = Begin(oldRuntime, oldManager, 1);
        ProgressReadinessState newRuntime = NewReadyState();
        int oldRuntimeResultReads = 0;
        int oldRuntimeIdentityReads = 0;
        False(CompleteWithReaders(
            newRuntime,
            oldAttempt,
            oldManager,
            17,
            () => { oldRuntimeResultReads++; return (true, 0, true); },
            () => { oldRuntimeIdentityReads++; return new ProgressReadinessHostIdentity(true, 1, new object()); }), "prior runtime incarnation refused");
        Equal(0, oldRuntimeResultReads, "prior runtime does not inspect host result");
        Equal(0, oldRuntimeIdentityReads, "prior runtime does not inspect host identity");

        ProgressReadinessState staleLoad = NewReadyState();
        object staleManager = new();
        ProgressReadinessLoadAttempt staleAttempt = Begin(staleLoad, staleManager, 1);
        staleLoad.BeforeProfileDeletion();
        int staleResultReads = 0;
        int staleIdentityReads = 0;
        False(CompleteWithReaders(
            staleLoad,
            staleAttempt,
            staleManager,
            17,
            () => { staleResultReads++; return (true, 0, true); },
            () => { staleIdentityReads++; return new ProgressReadinessHostIdentity(true, 1, new object()); }), "stale completion refused");
        Equal(0, staleResultReads, "stale completion does not inspect host result");
        Equal(0, staleIdentityReads, "stale completion does not inspect host identity");

        ProgressReadinessState missedStartup = NewReadyState();
        False(missedStartup.TryGetLease(17, out _), "late registration does not backfill earlier startup");
    }

    private static void VerifyRegistrationAndFinalizer()
    {
        False(ProgressReadinessHostPin.IsSupported(typeof(object).Assembly, typeof(object).Assembly), "unsupported host/Harmony identity refused");

        ProgressReadinessState state = new();
        FakeRegistry registry = new();
        ProgressReadinessPatchRegistration registration = new();
        True(registration.TryInstall(state, registry), "all exact hooks verified");
        Equal(5, registry.Installed.Count, "all lifecycle targets registered");
        for (int index = 0; index < ProgressReadinessPatchRegistration.Targets.Count; index++)
        {
            Equal(ProgressReadinessPatchRegistration.Targets[index], registry.Installed[index].Target, "exact registration target order");
        }
        True(registry.Installed.TrueForAll(item => item.Owner == ProgressReadinessPatchRegistration.OwnerId), "stable owner id applied");
        True(registration.TryInstall(state, registry), "repeat registration is idempotent");
        Equal(5, registry.Installed.Count, "repeat does not patch twice");

        ProgressReadinessState failedState = new();
        FakeRegistry missingOwner = new() { RefuseTarget = "SaveManager.DeleteProfile" };
        False(new ProgressReadinessPatchRegistration().TryInstall(failedState, missingOwner), "missing owner verification refused");
        False(failedState.IsOperational, "partial registration disables readiness");
        Equal(5, ProgressReadinessPatchRegistration.Targets.Count, "registration requires the complete lifecycle set");

        ProgressReadinessState throwingState = new();
        FakeRegistry throwingRegistry = new() { ThrowTarget = "SaveManager.InitProgressData" };
        False(new ProgressReadinessPatchRegistration().TryInstall(throwingState, throwingRegistry), "registration exception refused");
        False(throwingState.IsOperational, "registration exception disables readiness");

        ProgressReadinessState thrownState = new();
        False(OptionalProgressReadinessInstaller.TryInstall(
            thrownState,
            () => throw new TypeLoadException("synthetic missing Harmony binding")), "optional adapter load failure contained");
        False(thrownState.IsOperational, "optional adapter failure disables only readiness");

        ProgressReadinessState faultedState = NewReadyState();
        object manager = new();
        ProgressReadinessLoadAttempt attempt = Begin(faultedState, manager, 1);
        Exception hostException = new InvalidOperationException("synthetic original host exception");
        True(ReferenceEquals(hostException, ProgressReadinessCallbacks.FinalizeProgressLoad(faultedState, hostException, attempt)), "finalizer preserves original exception");
        False(faultedState.TryGetLease(17, out _), "exception revokes attempted readiness");
    }

    private static ProgressReadinessState NewReadyState()
    {
        ProgressReadinessState state = new();
        state.MarkObserverAttached();
        True(state.ObserveProcessFrame(17), "actual frame observation establishes owner");
        state.MarkPatchesVerified();
        return state;
    }

    private static ProgressReadinessLoadAttempt Begin(
        ProgressReadinessState state,
        object manager,
        int profileId,
        int threadId = 17)
    {
        return ProgressReadinessCallbacks.BeginProgressLoad(
            state,
            manager,
            threadId,
            () => new ProgressReadinessHostIdentity(true, profileId, null));
    }

    private static bool Publish(
        ProgressReadinessState state,
        ProgressReadinessLoadAttempt attempt,
        object instance,
        int profileId,
        object? before,
        object? after,
        bool success,
        int status,
        bool savedData,
        int threadId = 17)
    {
        int reads = 0;
        return CompleteWithReaders(
            state,
            attempt,
            instance,
            threadId,
            () => (success, status, savedData),
            () =>
            {
                object? progress = reads++ == 0 ? before : after;
                return new ProgressReadinessHostIdentity(true, profileId, progress);
            });
    }

    private static bool CompleteWithReaders(
        ProgressReadinessState state,
        ProgressReadinessLoadAttempt attempt,
        object instance,
        int threadId,
        Func<(bool Success, int Status, bool SavedDataPresent)> readResult,
        Func<ProgressReadinessHostIdentity> readIdentity) =>
        ProgressReadinessCallbacks.CompleteProgressLoad(
            state,
            attempt,
            instance,
            threadId,
            readResult,
            readIdentity);

    private static ProgressReadinessSnapshot Read(
        ProgressReadinessState state,
        object manager,
        object progress,
        int profileId)
    {
        True(TryCapture(state, manager, progress, profileId, out ProgressReadinessSnapshot snapshot), "validated lease copied");
        return snapshot;
    }

    private static bool TryCapture(
        ProgressReadinessState state,
        object manager,
        object progress,
        int profileId) => TryCapture(state, manager, progress, profileId, out _);

    private static bool TryCapture(
        ProgressReadinessState state,
        object manager,
        object progress,
        int profileId,
        out ProgressReadinessSnapshot snapshot)
    {
        return ProgressReadinessCallbacks.TryCaptureOwnedSnapshot(
            state,
            17,
            instance => new ProgressReadinessHostIdentity(
                ReferenceEquals(instance, manager),
                profileId,
                progress),
            out snapshot);
    }

    private static void AssertResult(bool success, int status, bool savedData, bool expected, string label)
    {
        ProgressReadinessState state = NewReadyState();
        object manager = new();
        ProgressReadinessLoadAttempt attempt = Begin(state, manager, 1);
        object stableProgress = new();
        bool published = Publish(state, attempt, manager, 1, stableProgress, stableProgress, success, status, savedData);
        Equal(expected, published, $"{label} publish result");
        Equal(expected, state.TryGetLease(17, out _), $"{label} readiness result");
    }

    private static void True(bool value, string label)
    {
        _checks++;
        if (!value) throw new InvalidOperationException($"failed: {label}");
    }

    private static void False(bool value, string label) => True(!value, label);

    private static void Equal<T>(T expected, T actual, string label) where T : notnull =>
        True(EqualityComparer<T>.Default.Equals(expected, actual), label);

    private sealed class FakeRegistry : IProgressReadinessPatchRegistry
    {
        internal readonly List<(string Target, string Owner)> Installed = new();
        internal string? RefuseTarget { get; init; }
        internal string? ThrowTarget { get; init; }

        public bool InstallAndVerifyOwner(string target, string ownerId)
        {
            if (target == ThrowTarget) throw new InvalidOperationException("synthetic Harmony registration failure");
            if (target == RefuseTarget) return false;
            Installed.Add((target, ownerId));
            return true;
        }
    }
}
