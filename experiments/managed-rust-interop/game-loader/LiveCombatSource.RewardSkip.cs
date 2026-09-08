// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.CardRewardAlternatives;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Rewards;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private const BindingFlags RewardFields = BindingFlags.Instance | BindingFlags.NonPublic;
    private static readonly FieldInfo? RewardOptionsField =
        typeof(NCardRewardSelectionScreen).GetField("_options", RewardFields);
    private static readonly FieldInfo? RewardAlternativesField =
        typeof(NCardRewardSelectionScreen).GetField("_extraOptions", RewardFields);
    private static readonly FieldInfo? RewardCompletionField =
        typeof(NCardRewardSelectionScreen).GetField("_completionSource", RewardFields);
    private static readonly FieldInfo? RewardAlternativeNameField =
        typeof(NCardRewardAlternativeButton).GetField("_optionName", RewardFields);
    private static readonly Func<Task>? DefaultRewardSkipCallback = new CardRewardAlternative("Skip",
        PostAlternateCardRewardAction.EndSelectionAndDoNotCompleteReward).OnSelect;

    private static bool IsPlainRewardSkipAlternative(CardRewardAlternative option) =>
        option.OptionId == "Skip"
        && option.AfterSelected == PostAlternateCardRewardAction.EndSelectionAndDoNotCompleteReward
        && DefaultRewardSkipCallback is not null && DefaultRewardSkipCallback.Equals(option.OnSelect);

    private sealed record RewardSkipChoice(NCardRewardAlternativeButton Button, Task<int?> Completion,
        int Selection, CardCreationResult[] Cards, CardRewardAlternative[] Alternatives)
    {
        internal bool Matches(RewardSkipChoice other) => ReferenceEquals(Button, other.Button)
            && ReferenceEquals(Completion, other.Completion) && Selection == other.Selection
            && Cards.SequenceEqual(other.Cards) && Alternatives.SequenceEqual(other.Alternatives);
    }

    private static bool TryRewardSkip(NCardRewardSelectionScreen screen, out RewardSkipChoice? choice)
    {
        choice = null;
        if (RewardOptionsField?.GetValue(screen) is not IReadOnlyList<CardCreationResult> cards
            || RewardAlternativesField?.GetValue(screen) is not IReadOnlyList<CardRewardAlternative> alternatives
            || RewardCompletionField?.GetValue(screen) is not TaskCompletionSource<int?> completion
            || RewardAlternativeNameField is null || completion.Task.IsCompleted
            || cards.Count is < 1 or > 64 || alternatives.Count is < 1 or > 2
            || alternatives.Any(option => option is null)) return false;
        var buttons = Descendants(screen).OfType<NCardRewardAlternativeButton>().Where(Clickable)
            .Select(button => (Button: button, Id: RewardAlternativeNameField.GetValue(button) as string))
            .ToArray();
        if (buttons.Any(button => button.Id is null)) return false;
        var values = alternatives.Select(option => new RuntimeV3GameplayRewardAlternative(option.OptionId,
            IsPlainRewardSkipAlternative(option))).ToArray();
        if (!RuntimeV3GameplayRewardSkip.TrySelection(values, buttons.Select(button => button.Id!).ToArray(),
            cards.Count, out int selection)) return false;
        choice = new(buttons.Single(button => button.Id == "Skip").Button, completion.Task,
            selection, cards.ToArray(), alternatives.ToArray());
        return true;
    }

    private static bool PrepareRewardSkip(NCardRewardSelectionScreen screen, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (!TryRewardSkip(screen, out RewardSkipChoice? choice) || choice is null
            || CurrentPlayer() is not { } player) return false;
        var deck = player.Deck.Cards.ToArray();
        invoke = () =>
        {
            if (!ReferenceEquals(RewardOverlay(), screen) || !TryRewardSkip(screen, out RewardSkipChoice? current)
                || current is null || !choice.Matches(current))
                throw new InvalidOperationException("native reward skip binding changed before execution");
            choice.Button.ForceClick();
            return Task.CompletedTask;
        };
        postcondition = () => RuntimeV3GameplayRewardSkip.Selected(choice.Completion, choice.Selection)
            && ReferenceEquals(CurrentPlayer(), player) && !ReferenceEquals(RewardOverlay(), screen)
            && deck.SequenceEqual(player.Deck.Cards);
        effect = "reward_card_selection_skipped";
        return true;
    }
}
