// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Security.Cryptography;
using System.Text;
using Godot;
using MegaCrit.Sts2.Core.CardSelection;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Cards;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private static bool TryReadSimpleCardGridCatalog(
        NSimpleCardSelectScreen screen,
        out SimpleCardGridCatalog? catalog,
        out string error)
    {
        catalog = null;
        error = "native_player_choice_catalog_unavailable";
        if (!GodotObject.IsInstanceValid(screen) || !screen.IsVisibleInTree())
            return false;

        if (SimplePrefsField?.GetValue(screen) is not CardSelectorPrefs preferences)
            return false;

        // NSimpleCardSelectScreen.Create(CardCreationResult, prefs) sorts a private copy when
        // Comparison is supplied. CardSelectCmd later indexes its original unsorted list, which
        // is no longer available from the screen. Refuse that case rather than guessing.
        if (preferences.Comparison is not null)
        {
            error = "native_player_choice_catalog_sorted_unsupported";
            return false;
        }

        NCardGrid? grid = screen.GetNodeOrNull<NCardGrid>("%CardGrid");
        if (grid is null || !GodotObject.IsInstanceValid(grid) || !grid.IsVisibleInTree())
            return false;
        if (GridCardsField?.GetValue(grid) is not IEnumerable<CardModel> cardValues)
            return false;

        CardModel[] cards = cardValues.ToArray();
        if (cards.Length == 0 || cards.Any(card => card is null))
            return false;

        if (!TryReadSelectedIndexes(screen, cards, out HashSet<int> selectedIndexes,
                out error))
        {
            return false;
        }

        NGridCardHolder[] holders;
        try
        {
            holders = grid.CurrentlyDisplayedCardHolders.ToArray();
        }
        catch
        {
            return false;
        }
        if (holders.Length == 0)
            return false;

        var visible = new Dictionary<int, NGridCardHolder>();
        int previousIndex = -1;
        foreach (NGridCardHolder holder in holders)
        {
            if (!GodotObject.IsInstanceValid(holder) || !Usable(holder.Hitbox)
                || holder.CardModel is not { } card)
            {
                error = "native_player_choice_card_unavailable";
                return false;
            }

            int index = IndexOfReference(cards, card);
            if (index < 0 || !visible.TryAdd(index, holder)
                || previousIndex >= 0 && index != previousIndex + 1)
            {
                // The base grid owns a contiguous row window. A different ordering means a
                // sorting or virtualization contract changed, so its result index is unsafe.
                error = "native_player_choice_catalog_order_unavailable";
                return false;
            }
            previousIndex = index;
        }

        string fingerprint = Fingerprint(screen, cards, visible.Keys, selectedIndexes);
        catalog = new SimpleCardGridCatalog(
            grid, cards, visible, selectedIndexes, fingerprint, preferences);
        error = string.Empty;
        return true;
    }

    private static bool TryReadSelectedIndexes(
        NSimpleCardSelectScreen screen,
        IReadOnlyList<CardModel> cards,
        out HashSet<int> selectedIndexes,
        out string error)
    {
        selectedIndexes = new HashSet<int>();
        error = "native_player_choice_selection_state_unavailable";
        IEnumerable<CardModel> selectedCards;
        try
        {
            if (SimpleSelectedCardsField?.GetValue(screen) is not IEnumerable<CardModel> value)
                return false;
            selectedCards = value;
        }
        catch
        {
            return false;
        }

        CardModel[] selected;
        try
        {
            selected = selectedCards.ToArray();
        }
        catch
        {
            return false;
        }

        foreach (CardModel card in selected)
        {
            int index = IndexOfReference(cards, card);
            if (index < 0 || !selectedIndexes.Add(index))
            {
                error = "native_player_choice_selection_state_invalid";
                return false;
            }
        }

        error = string.Empty;
        return true;
    }

    private static int IndexOfReference(IReadOnlyList<CardModel> cards, CardModel target)
    {
        for (int index = 0; index < cards.Count; index++)
        {
            if (ReferenceEquals(cards[index], target))
                return index;
        }
        return -1;
    }
}
