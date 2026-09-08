// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopAdmissionTests;

internal static class Program
{
    private static void Main()
    {
        Check(CoopNativeLobbyAdmission.IsOwnedAndStarted(true, true, true),
            "owned connected service with an active run settles admission");
        Check(!CoopNativeLobbyAdmission.IsOwnedAndStarted(true, true, false),
            "an unrelated active run cannot settle this controller");
        Check(!CoopNativeLobbyAdmission.IsOwnedAndStarted(false, true, true),
            "a connected service before run start cannot settle admission");

        var flow = new AdmissionFlowFixture();
        flow.Tick(runInProgress: false);
        Check(flow.SceneLookupCount == 1,
            "the pre-run frame waits on the scene lookup");
        Check(flow.DisposeCount == 0,
            "the pre-run frame does not dispose the controller");

        flow.Tick(runInProgress: true);
        Check(flow.SceneLookupCount == 1,
            "owned run admission returns before another scene lookup");
        Check(flow.DisposeCount == 1,
            "owned run admission completes and disposes exactly once");

        flow.Tick(runInProgress: true);
        Check(flow.SceneLookupCount == 1 && flow.DisposeCount == 1,
            "a completed admission does not re-enter scene lookup or disposal");
        Console.WriteLine("Native co-op admission transition tests passed.");
    }

    private sealed class AdmissionFlowFixture
    {
        internal int SceneLookupCount { get; private set; }
        internal int DisposeCount { get; private set; }
        private bool _completed;

        internal void Tick(bool runInProgress)
        {
            if (_completed)
                return;
            if (CoopNativeLobbyAdmission.TryAdmitOwnedRun(
                    autoAdmitRun: true,
                    runInProgress: runInProgress,
                    serviceConnected: true,
                    serviceMatchesOwner: true,
                    admit: Complete))
            {
                return;
            }
            FindSubmenuStack();
        }

        private void FindSubmenuStack() => SceneLookupCount++;

        private void Complete()
        {
            _completed = true;
            Dispose();
        }

        private void Dispose() => DisposeCount++;
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
