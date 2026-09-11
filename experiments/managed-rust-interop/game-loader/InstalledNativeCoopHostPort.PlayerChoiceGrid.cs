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
}
