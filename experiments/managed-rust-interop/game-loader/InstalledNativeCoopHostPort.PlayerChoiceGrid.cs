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
    private static readonly FieldInfo? GridCardsField = typeof(NCardGrid).GetField(
        "_cards", BindingFlags.Instance | BindingFlags.NonPublic);

    private static readonly FieldInfo? SimplePrefsField = typeof(NSimpleCardSelectScreen).GetField(
        "_prefs", BindingFlags.Instance | BindingFlags.NonPublic);

    private static readonly FieldInfo? SimpleSelectedCardsField = typeof(NSimpleCardSelectScreen).GetField(
        "_selectedCards", BindingFlags.Instance | BindingFlags.NonPublic);

    private sealed record SimpleCardGridCatalog(
        NCardGrid Grid,
        IReadOnlyList<CardModel> Cards,
        IReadOnlyDictionary<int, NGridCardHolder> VisibleHolders,
        IReadOnlySet<int> SelectedIndexes,
        string Fingerprint,
        CardSelectorPrefs Preferences);

    /// <summary>
    /// Returns the active visible choice catalog identity used by the native host-generation
    /// fence. The simple grid is the only generic grid whose wire result is an index and whose
    /// source order can be proven from the installed first-party implementation.
    /// </summary>
    private static string PlayerChoiceCatalogFingerprint()
    {
        Node? screen = VisibleChoiceScreen();
        if (screen is not NSimpleCardSelectScreen simple)
        {
            return screen is null
                ? "none"
                : "screen:" + screen.GetType().FullName;
        }

        return TryReadSimpleCardGridCatalog(simple, out SimpleCardGridCatalog? catalog, out string error)
            ? "simple:" + catalog!.Fingerprint
            : "simple:unavailable:" + error;
    }

    private static bool TryChooseSimpleCardGrid(
        NSimpleCardSelectScreen screen, List<int> indexes, out string error)
    {
        error = "native_player_choice_catalog_unavailable";
        if (!TryReadSimpleCardGridCatalog(screen, out SimpleCardGridCatalog? catalog,
                out error) || catalog is null)
        {
            return false;
        }

        // The first-party holder signal toggles membership in _selectedCards. Native indexes
        // describe the desired final result, so a non-empty local baseline cannot be safely
        // replayed by blindly emitting Pressed signals. Keep the state in the catalog fingerprint
        // and require a known empty baseline before applying a native choice.
        if (catalog.SelectedIndexes.Count != 0)
        {
            error = "native_player_choice_selection_preselected";
            return false;
        }

        if (!TryReadSelectedIndexes(screen, catalog.Cards, out HashSet<int> currentSelection,
                out string currentSelectionError))
        {
            error = currentSelectionError;
            return false;
        }
        if (currentSelection.Count != 0)
        {
            error = "native_player_choice_selection_preselected";
            return false;
        }

        if (indexes.Count > 256 || indexes.Distinct().Count() != indexes.Count)
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        if (indexes.Any(index => index < 0 || index >= catalog.Cards.Count))
        {
            error = "native_player_choice_index_out_of_range";
            return false;
        }

        if (indexes.Count < catalog.Preferences.MinSelect
            || indexes.Count > catalog.Preferences.MaxSelect)
        {
            error = "native_player_choice_selection_incomplete";
            return false;
        }

        var selected = new List<NGridCardHolder>(indexes.Count);
        foreach (int index in indexes)
        {
            if (!catalog.VisibleHolders.TryGetValue(index, out NGridCardHolder? holder))
            {
                // Virtualized rows that are outside the current visible window have no safe
                // synchronous UI target. A caller can refresh its catalog and scroll through the
                // first-party UI before trying the operation again.
                error = "native_player_choice_card_not_visible";
                return false;
            }
            if (!Usable(holder.Hitbox))
            {
                error = "native_player_choice_card_unavailable";
                return false;
            }
            selected.Add(holder);
        }

        int clicked = 0;
        foreach (NGridCardHolder holder in selected)
        {
            if (!screen.IsVisibleInTree() || !GodotObject.IsInstanceValid(holder)
                || !Usable(holder.Hitbox))
            {
                error = clicked == 0
                    ? "native_player_choice_ui_unavailable"
                    : "native_player_choice_dispatch_outcome_unknown";
                return false;
            }

            if (holder.EmitSignal(NCardHolder.SignalName.Pressed, holder) != Error.Ok)
            {
                error = clicked == 0
                    ? "native_player_choice_ui_dispatch_failed"
                    : "native_player_choice_dispatch_outcome_unknown";
                return false;
            }
            clicked++;
        }

        // A fixed-size, non-manual selector closes from its last holder press. Variable-size
        // selectors keep the grid open and expose the first-party confirmation button instead.
        if (!screen.IsVisibleInTree())
        {
            error = string.Empty;
            return true;
        }

        if (!TryReadSelectedIndexes(screen, catalog.Cards, out HashSet<int> selectedIndexes,
                out string selectionError))
        {
            error = clicked == 0
                ? selectionError
                : "native_player_choice_dispatch_outcome_unknown";
            return false;
        }
        if (!selectedIndexes.SetEquals(indexes))
        {
            error = clicked == 0
                ? "native_player_choice_selection_mismatch"
                : "native_player_choice_dispatch_outcome_unknown";
            return false;
        }

        NConfirmButton? confirm = screen.GetNodeOrNull<NConfirmButton>("%Confirm");
        if (confirm is null || !GodotObject.IsInstanceValid(confirm))
        {
            error = "native_player_choice_confirmation_unavailable";
            return false;
        }
        if (!Usable(confirm))
        {
            error = "native_player_choice_selection_incomplete";
            return false;
        }

        confirm.ForceClick();
        error = string.Empty;
        return true;
    }

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

    private static string Fingerprint(
        NSimpleCardSelectScreen screen,
        CardModel[] cards,
        IEnumerable<int> visibleIndexes,
        IEnumerable<int> selectedIndexes)
    {
        var payload = new StringBuilder(screen.GetType().FullName);
        payload.Append('|').Append(cards.Length).Append('|');
        for (int index = 0; index < cards.Length; index++)
        {
            CardModel card = cards[index];
            payload.Append(index).Append(':').Append(card.Id).Append(':')
                .Append(RuntimeHelpers.GetHashCode(card)).Append(';');
        }
        payload.Append("visible:");
        foreach (int index in visibleIndexes.OrderBy(index => index))
            payload.Append(index).Append(',');
        payload.Append("|selected:");
        foreach (int index in selectedIndexes.OrderBy(index => index))
            payload.Append(index).Append(',');
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(payload.ToString())))
            .ToLowerInvariant();
    }
}
