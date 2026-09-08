// SPDX-License-Identifier: MIT

using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.CoopHostTests;

internal static partial class Program
{
    private static void RejoinAttemptWindowResetsAfterLateReconnectWithoutExtension()
    {
        const long frequency = 10;
        const int maxSeconds = 3;
        NativeCoopAttemptWindow window = new(100, frequency, maxSeconds);

        Check(window.DeadlineTimestamp == 130
            && !window.IsExpired(129)
            && window.IsExpired(130),
            "initial native attempt uses an absolute monotonic deadline");

        // A reconnect that arrives after the original bootstrap window gets a fresh bounded
        // attempt, rather than inheriting an already expired deadline.
        Check(window.TryReset(attemptInProgress: false, nowTimestamp: 200)
            && window.DeadlineTimestamp == 230
            && !window.IsExpired(229)
            && window.IsExpired(230),
            "late rejoin starts a fresh bounded attempt window");

        // RequestRejoin returns before TryResetForRejoinAttempt for an in-flight duplicate. The
        // guarded reset itself also refuses an active attempt, so this invariant remains local to
        // the timing primitive if a future caller moves the surrounding branch.
        Check(!window.TryReset(attemptInProgress: true, nowTimestamp: 220)
            && window.DeadlineTimestamp == 230
            && window.IsExpired(230),
            "repeated in-flight rejoin does not extend its deadline");
    }
}
