// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Owns the bounded monotonic-clock window for one native co-op attempt. Keeping the arithmetic
/// here makes late reconnect behavior deterministic to verify without loading the game assembly.
/// </summary>
internal sealed class NativeCoopAttemptWindow
{
    private readonly long _durationTicks;

    internal NativeCoopAttemptWindow(long nowTimestamp, long frequency, int maxSeconds)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(frequency);
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maxSeconds);

        _durationTicks = checked((long)maxSeconds * frequency);
        DeadlineTimestamp = checked(nowTimestamp + _durationTicks);
    }

    internal long DeadlineTimestamp { get; private set; }

    internal bool IsExpired(long nowTimestamp) => nowTimestamp >= DeadlineTimestamp;

    internal bool TryReset(bool attemptInProgress, long nowTimestamp)
    {
        if (attemptInProgress)
            return false;
        DeadlineTimestamp = checked(nowTimestamp + _durationTicks);
        return true;
    }
}
