// SPDX-License-Identifier: MIT

using System;
using System.Threading.Tasks;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class RewardSkipChecks
{
    private static readonly string[] SkipButton = ["Skip"];

    internal static void Run()
    {
        var alternatives = new[] { new RuntimeV3GameplayRewardAlternative("REROLL", false),
            new RuntimeV3GameplayRewardAlternative("Skip", true) };
        if (!RuntimeV3GameplayRewardSkip.TrySelection(alternatives, SkipButton, 3, out int selected)
            || selected != 4)
            throw new InvalidOperationException("skip must retain the native alternate index after the cards");
        foreach (string[] buttons in new[] { Array.Empty<string>(), new[] { "REROLL" },
            new[] { "Skip", "Skip" } })
            if (RuntimeV3GameplayRewardSkip.TrySelection(alternatives, buttons, 3, out _))
                throw new InvalidOperationException("absent or ambiguous skip control must fail closed");
        foreach (var options in new[] {
            new[] { new RuntimeV3GameplayRewardAlternative("Skip", false) },
            new[] { new RuntimeV3GameplayRewardAlternative("Skip", true),
                new RuntimeV3GameplayRewardAlternative("Skip", true) },
            Array.Empty<RuntimeV3GameplayRewardAlternative>() })
            if (RuntimeV3GameplayRewardSkip.TrySelection(options, SkipButton, 3, out _))
                throw new InvalidOperationException("only a unique plain native skip may be admitted");
        foreach (int count in new[] { -1, 0, 65, int.MaxValue })
            if (RuntimeV3GameplayRewardSkip.TrySelection(alternatives, SkipButton, count, out _))
                throw new InvalidOperationException("card count must be bounded before encoding selection");
        var pending = new TaskCompletionSource<int?>();
        if (RuntimeV3GameplayRewardSkip.Selected(pending.Task, selected)
            || RuntimeV3GameplayRewardSkip.Selected(Task.FromResult<int?>(null), selected)
            || RuntimeV3GameplayRewardSkip.Selected(Task.FromResult<int?>(3), selected)
            || RuntimeV3GameplayRewardSkip.Selected(Task.FromException<int?>(new InvalidOperationException()), selected)
            || RuntimeV3GameplayRewardSkip.Selected(Task.FromCanceled<int?>(new(true)), selected))
            throw new InvalidOperationException("unresolved, wrong, cancelled or failed selections do not settle skip");
        pending.SetResult(selected);
        if (!RuntimeV3GameplayRewardSkip.Selected(pending.Task, selected))
            throw new InvalidOperationException("the retained exact native selection must be observable");
    }
}
