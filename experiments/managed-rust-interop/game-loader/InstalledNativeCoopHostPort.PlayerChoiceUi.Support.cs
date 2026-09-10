// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Entities.Models;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;
using MegaCrit.Sts2.Core.Nodes.Screens.Overlays;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private static bool TryChoosePlayerTarget(
        PlayerChoiceResult result, out string error)
    {
        error = "native_player_choice_target_ui_unsupported";
        ulong? playerId;
        try
        {
            playerId = result.AsPlayerId();
        }
        catch
        {
            error = "native_player_choice_result_invalid";
            return false;
        }

        if (playerId is not { } targetId)
        {
            error = "native_player_choice_target_unavailable";
            return false;
        }

        // PlayerChoiceType.Player is currently emitted by the rest-site Mend selector. Its
        // completion is owned by NTargetManager: OnNodeHovered establishes the legal hovered
        // node and the public _Input override runs the same release path as a first-party click.
        // This avoids private-field access and lets the native selector resolve its own task.
        NRestSiteRoom? room = NRestSiteRoom.Instance;
        NTargetManager? targetManager = NTargetManager.Instance;
        if (room is null || targetManager is null
            || !GodotObject.IsInstanceValid(room)
            || !GodotObject.IsInstanceValid(targetManager)
            || !room.IsVisibleInTree() || !targetManager.IsInSelection)
        {
            return false;
        }
        NTargetManager targetManagerValue = targetManager;

        NRestSiteCharacter[] matches = room.Characters
            .Where(character => GodotObject.IsInstanceValid(character)
                && character.IsVisibleInTree()
                && character.Player.NetId == targetId
                && targetManagerValue.AllowedToTargetNode(character))
            .ToArray();
        if (matches.Length != 1)
        {
            error = matches.Length == 0
                ? "native_player_choice_target_unavailable"
                : "native_player_choice_target_ambiguous";
            return false;
        }

        try
        {
            targetManagerValue.OnNodeHovered(matches[0]);
            targetManagerValue._Input(new InputEventMouseButton
            {
                ButtonIndex = MouseButton.Left,
                Pressed = false
            });
            error = string.Empty;
            return true;
        }
        catch
        {
            error = "native_player_choice_ui_dispatch_failed";
            return false;
        }
    }

    private static bool AllCardHoldersUsable(IEnumerable<NGridCardHolder> holders) =>
        holders.Any() && holders.All(holder => Usable(holder.Hitbox));

    private static bool Usable(NClickableControl control) =>
        GodotObject.IsInstanceValid(control) && control.IsVisibleInTree() && control.IsEnabled;

    private static Node? VisibleChoiceScreen()
    {
        try
        {
            NOverlayStack? stack = NOverlayStack.Instance;
            if (stack is not null && stack.ScreenCount > 0 && stack.Peek() is Node screen
                && GodotObject.IsInstanceValid(screen)
                && screen is CanvasItem canvas && canvas.IsVisibleInTree())
                return screen;

            NGame? game = NGame.Instance;
            if (game is null || !GodotObject.IsInstanceValid(game))
                return null;
            Node[] candidates = Descendants(game)
                .Where(node => node is NChooseACardSelectionScreen
                    or NChooseARelicSelection
                    or NCardRewardSelectionScreen
                    or NChooseABundleSelectionScreen
                    or NCardGridSelectionScreen)
                .Where(node => node is CanvasItem canvas && canvas.IsVisibleInTree())
                .ToArray();
            return candidates.Length == 1 ? candidates[0] : null;
        }
        catch
        {
            return null;
        }
    }

    private static IEnumerable<Node> Descendants(Node root)
    {
        foreach (Node child in root.GetChildren())
        {
            if (!GodotObject.IsInstanceValid(child))
                continue;
            yield return child;
            foreach (Node descendant in Descendants(child))
                yield return descendant;
        }
    }
}
