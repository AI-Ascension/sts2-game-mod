// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Entities.Models;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.Relics;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;
using MegaCrit.Sts2.Core.Nodes.Screens.Overlays;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Dispatches a native player-choice request through the installed game's visible choice
/// surface. PlayerChoiceSynchronizer's local route only broadcasts a result; it does not finish
/// the local selection task, so calling it directly would leave a local action paused forever.
/// The UI callbacks below complete that first-party task and let the game serialize the result.
/// </summary>
internal sealed partial class InstalledNativeCoopHostPort
{
    /// <summary>
    /// Performs the final catalog and control checks immediately before a choice is dispatched.
    /// The method deliberately supports only screens whose public scene-tree children preserve
    /// the result index used by the corresponding first-party command. A grid with a private,
    /// sorted, or virtualized catalog remains unavailable rather than accepting an index that
    /// cannot be mapped to the command's original list.
    /// </summary>
    private static bool TryDispatchPlayerChoiceThroughUi(
        PlayerChoiceResult result, out string error)
    {
        error = "native_player_choice_ui_unavailable";
        if (result.ChoiceType == PlayerChoiceType.Player)
            return TryChoosePlayerTarget(result, out error);

        Node? screen = VisibleChoiceScreen();
        if (screen is null)
            return false;

        List<int> indexes;
        try
        {
            indexes = result.AsIndexes();
        }
        catch
        {
            error = "native_player_choice_result_invalid";
            return false;
        }

        if (screen is NChooseACardSelectionScreen chooseCard)
            return TryChooseSingleCard(chooseCard, indexes, out error);
        if (screen is NChooseARelicSelection chooseRelic)
            return TryChooseSingleRelic(chooseRelic, indexes, out error);
        if (screen is NCardRewardSelectionScreen cardReward)
            return TryChooseCardReward(cardReward, indexes, out error);
        if (screen is NChooseABundleSelectionScreen bundle)
            return TryChooseBundle(bundle, indexes, out error);

        if (screen is NSimpleCardSelectScreen simpleGrid)
            return TryChooseSimpleCardGrid(simpleGrid, indexes, out error);

        // Other NCardGridSelectionScreen subclasses serialize mutable card identities rather than
        // index results. The native vote wire has no such identity, so leave those selectors
        // unavailable instead of sending an index that the waiting command will misinterpret.
        error = "native_player_choice_catalog_unavailable";
        return false;
    }

    private static bool TryChooseSingleCard(
        NChooseACardSelectionScreen screen, List<int> indexes, out string error)
    {
        error = "native_player_choice_catalog_unavailable";
        Control? row = screen.GetNodeOrNull<Control>("CardRow");
        if (row is null || !row.IsVisibleInTree())
            return false;
        NGridCardHolder[] holders = Descendants(row).OfType<NGridCardHolder>().ToArray();
        if (holders.Length == 0 || !AllCardHoldersUsable(holders))
            return false;

        if (indexes.Count == 0)
        {
            NChoiceSelectionSkipButton? skip = screen.GetNodeOrNull<NChoiceSelectionSkipButton>(
                "SkipButton");
            if (skip is null || !Usable(skip))
            {
                error = "native_player_choice_skip_unavailable";
                return false;
            }
            skip.ForceClick();
            error = string.Empty;
            return true;
        }

        if (indexes.Count != 1 || indexes[0] < 0 || indexes[0] >= holders.Length)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        Error emitted = holders[indexes[0]].EmitSignal(
            NCardHolder.SignalName.Pressed, holders[indexes[0]]);
        if (emitted != Error.Ok)
        {
            error = "native_player_choice_ui_dispatch_failed";
            return false;
        }
        error = string.Empty;
        return true;
    }

    private static bool TryChooseSingleRelic(
        NChooseARelicSelection screen, List<int> indexes, out string error)
    {
        error = "native_player_choice_catalog_unavailable";
        Control? row = screen.GetNodeOrNull<Control>("RelicRow");
        if (row is null || !row.IsVisibleInTree())
            return false;
        NRelicBasicHolder[] holders = Descendants(row).OfType<NRelicBasicHolder>().ToArray();
        if (holders.Length == 0 || !holders.All(Usable))
            return false;

        if (indexes.Count == 0)
        {
            NChoiceSelectionSkipButton? skip = screen.GetNodeOrNull<NChoiceSelectionSkipButton>(
                "SkipButton");
            if (skip is null || !Usable(skip))
            {
                error = "native_player_choice_skip_unavailable";
                return false;
            }
            skip.ForceClick();
            error = string.Empty;
            return true;
        }

        if (indexes.Count != 1 || indexes[0] < 0 || indexes[0] >= holders.Length)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        holders[indexes[0]].ForceClick();
        error = string.Empty;
        return true;
    }

    private static bool TryChooseCardReward(
        NCardRewardSelectionScreen screen, List<int> indexes, out string error)
    {
        error = "native_player_choice_catalog_unavailable";
        if (indexes.Count != 1)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        Control? row = screen.GetNodeOrNull<Control>("UI/CardRow");
        Control? alternatives = screen.GetNodeOrNull<Control>("UI/RewardAlternatives");
        if (row is null || alternatives is null || !row.IsVisibleInTree()
            || !alternatives.IsVisibleInTree())
            return false;

        NGridCardHolder[] cards = Descendants(row).OfType<NGridCardHolder>().ToArray();
        NCardRewardAlternativeButton[] extra = Descendants(alternatives)
            .OfType<NCardRewardAlternativeButton>().ToArray();
        // A reward can contain only alternatives (for example a transformed or already
        // exhausted reward), so an empty card row is legal when at least one alternative is
        // present. Require a non-empty combined catalog and validate each rendered control.
        if (cards.Length + extra.Length == 0 || !cards.All(card => Usable(card.Hitbox))
            || !extra.All(Usable)
            || indexes[0] < 0 || indexes[0] >= cards.Length + extra.Length)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        if (indexes[0] < cards.Length)
        {
            Error emitted = cards[indexes[0]].EmitSignal(
                NCardHolder.SignalName.Pressed, cards[indexes[0]]);
            if (emitted != Error.Ok)
            {
                error = "native_player_choice_ui_dispatch_failed";
                return false;
            }
        }
        else
        {
            extra[indexes[0] - cards.Length].ForceClick();
        }
        error = string.Empty;
        return true;
    }

    private static bool TryChooseBundle(
        NChooseABundleSelectionScreen screen, List<int> indexes, out string error)
    {
        error = "native_player_choice_catalog_unavailable";
        if (indexes.Count != 1)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        Control? row = screen.GetNodeOrNull<Control>("%BundleRow");
        Control? preview = screen.GetNodeOrNull<Control>("%BundlePreviewContainer");
        NConfirmButton? confirm = screen.GetNodeOrNull<NConfirmButton>("%PreviewConfirm");
        if (row is null || preview is null || confirm is null || !row.IsVisibleInTree()
            || preview.Visible || !confirm.IsVisibleInTree())
            return false;

        NCardBundle[] bundles = Descendants(row).OfType<NCardBundle>().ToArray();
        if (bundles.Length == 0 || !bundles.All(bundle => Usable(bundle.Hitbox))
            || indexes[0] < 0 || indexes[0] >= bundles.Length)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        bundles[indexes[0]].Hitbox.ForceClick();
        if (!preview.Visible || !Usable(confirm))
        {
            error = "native_player_choice_bundle_confirmation_unavailable";
            return false;
        }
        confirm.ForceClick();
        error = string.Empty;
        return true;
    }

}
