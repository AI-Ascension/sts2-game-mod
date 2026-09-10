// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Helpers;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Game.Lobby;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;
using MegaCrit.Sts2.Core.Random;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class SeededRunStandardHost
{
    private static bool TryPrepare(
        SeededRunStandardRequest request,
        out NCharacterSelectScreen screen,
        out string canonicalSeed,
        out SeededRunStandardCompatibilitySnapshot? compatibility,
        out ulong generationBefore,
        out string error)
    {
        screen = null!;
        canonicalSeed = string.Empty;
        compatibility = null;
        generationBefore = 0;
        error = string.Empty;

        SeededRunSelectionContext context = request.SelectedContext;
        if (context.ProfileBaseline.Kind != SeededRunSelectionContext.FreshProfileKind
            || context.SavePolicy != SeededRunSelectionContext.EnabledSavePolicy
            || context.SelectionPolicy != NativeSelectionPolicy
            || context.Ascension != 0
            || context.Modifiers.Count != 0)
        {
            error = "unsupported_standard_context";
            return false;
        }

        if (!SeededRunProfileBaseline.Matches(context.ProfileBaseline, out error))
        {
            return false;
        }

        SaveManager saves = SaveManager.Instance;
        if (!saves.IsProfileInitialized || saves.CurrentProfileId is < 1 or > 3)
        {
            error = "profile_unavailable";
            return false;
        }

        try
        {
            LiveCampaignStart.LoadProgress();
        }
        catch (Exception exception)
        {
            error = "progress_unavailable_" + exception.GetType().Name;
            return false;
        }

        if (saves.Progress.NumberOfRuns != 0)
        {
            error = "profile_not_fresh";
            return false;
        }

        if (saves.HasRunSave)
        {
            error = "run_save_exists";
            return false;
        }

        if (saves.CurrentRunSaveTask is { IsCompleted: false })
        {
            error = "run_save_pending";
            return false;
        }

        RunManager manager = RunManager.Instance;
        if (manager.IsInProgress || manager.IsCleaningUp || manager.DebugOnlyGetState() is not null)
        {
            error = "run_in_progress";
            return false;
        }

        NGame? game = NGame.Instance;
        if (game is null || game.MainMenu is null || game.DebugSeedOverride is not null)
        {
            error = "host_not_ready";
            return false;
        }

        try
        {
            compatibility = SeededRunStandardCompatibilitySnapshot.Current();
        }
        catch (Exception exception)
        {
            error = "compatibility_unavailable_" + exception.GetType().Name;
            return false;
        }

        if (!LiveCombatSource.TryReadCurrentGeneration(out generationBefore))
        {
            error = "runtime_generation_unavailable";
            return false;
        }

        if (context.Compatibility.Game.Identity != compatibility.GameIdentity
            || context.Compatibility.Game.Digest != compatibility.GameDigest
            || context.Compatibility.Mod.Identity != compatibility.ModIdentity
            || context.Compatibility.Mod.Digest != compatibility.ModDigest)
        {
            error = "compatibility_mismatch";
            return false;
        }

        try
        {
            var stack = game.MainMenu.SubmenuStack;
            screen = stack.GetSubmenuType<NCharacterSelectScreen>();
            screen.InitializeSingleplayer();
            stack.PushSubmenuType<NCharacterSelectScreen>();
            if (screen.Lobby.GameMode != GameMode.Standard
                || screen.Lobby.NetService.Type != NetGameType.Singleplayer
                || screen.Lobby.Players.Count != 1
                || screen.Lobby.Ascension != context.Ascension
                || screen.Lobby.Modifiers.Count != 0
                || screen.Lobby.Act1 != "random")
            {
                error = "native_standard_lobby_mismatch";
                screen = null!;
                return false;
            }

            canonicalSeed = SeedHelper.CanonicalizeSeed(request.RequestedSeed);
            if (!SeededRunStandardContract.IsSeed(canonicalSeed))
            {
                error = "canonical_seed_invalid";
                screen = null!;
                return false;
            }

            if (!NativeActsMatch(screen.Lobby, canonicalSeed, context.Acts, out error))
            {
                screen = null!;
                return false;
            }

            return true;
        }
        catch (Exception exception)
        {
            error = "native_lobby_unavailable_" + exception.GetType().Name;
            screen = null!;
            return false;
        }
    }

    /// <summary>
    /// Replays the native standard lobby's read-only act selection calculation before SetReady.
    /// StartRunLobby.BeginRunLocally uses these same inputs after admission; computing them here
    /// prevents an incorrect selected_context from reaching the mutation boundary.
    /// </summary>
    private static bool NativeActsMatch(
        StartRunLobby lobby,
        string canonicalSeed,
        IReadOnlyList<string> expectedActs,
        out string error)
    {
        error = string.Empty;
        if (lobby.Act1 != "random")
        {
            error = "native_selection_policy_mismatch";
            return false;
        }

        try
        {
            var rng = new Rng(
                (uint)StringHelper.GetDeterministicHashCode(canonicalSeed),
                "act_selection");
            var nativeActs = ActModel.GetRandomList(
                    rng,
                    new MegaCrit.Sts2.Core.Unlocks.UnlockState(SaveManager.Instance.Progress),
                    lobby.NetService.Type.IsMultiplayer())
                .Select(act => act.Id.Entry)
                .ToArray();
            if (!nativeActs.SequenceEqual(expectedActs, StringComparer.Ordinal))
            {
                error = "native_acts_mismatch";
                return false;
            }

            return true;
        }
        catch (Exception exception)
        {
            error = "native_acts_unavailable_" + exception.GetType().Name;
            return false;
        }
    }
}
