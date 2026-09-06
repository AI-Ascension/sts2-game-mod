// SPDX-License-Identifier: MIT

using System;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class LiveCampaignStart
{
    internal static async Task<RunState> StandardAsync(CharacterModel? requestedCharacter = null)
    {
        LoadProgress();
        if (SaveManager.Instance.HasRunSave)
            throw new InvalidOperationException("existing campaign save requires explicit resume");
        NGame game = NGame.Instance ?? throw new InvalidOperationException("game unavailable");
        var stack = (game.MainMenu ?? throw new InvalidOperationException("main menu unavailable")).SubmenuStack;
        var screen = stack.GetSubmenuType<NCharacterSelectScreen>();
        screen.InitializeSingleplayer();
        stack.PushSubmenuType<NCharacterSelectScreen>();
        if (screen.Lobby.GameMode != GameMode.Standard)
            throw new InvalidOperationException("host character lobby is not standard mode");
        CharacterModel character = requestedCharacter ?? ModelDb.Character<Ironclad>();
        if (!character.IsPlayable)
            throw new InvalidOperationException("requested character is not playable");
        screen.Lobby.SetLocalCharacter(character);
        // The host lobby owns random seed, act selection, and the normal saving-enabled start.
        screen.Lobby.SetReady(true);
        for (int frame = 0; frame < 1800; frame++)
        {
            if (RunManager.Instance.IsInProgress && RunManager.Instance.ShouldSave
                && RunManager.Instance.DebugOnlyGetState() is { GameMode: GameMode.Standard } run)
                return run;
            await game.ToSignal(game.GetTree(), SceneTree.SignalName.ProcessFrame);
        }
        throw new InvalidOperationException("normal campaign startup did not complete");
    }

    internal static async Task ResumeAsync()
    {
        LoadProgress();
        var saved = SaveManager.Instance.LoadRunSave();
        if (!saved.Success || saved.SaveData is not { GameMode: GameMode.Standard } data)
            throw new InvalidOperationException("standard campaign resume save is unavailable");
        RunState run = RunState.FromSerializable(data);
        await RunManager.Instance.SetUpSavedSingleplayer(run, data);
        await NGame.Instance!.LoadRun(run, data.PreFinishedRoom);
        if (!RunManager.Instance.ShouldSave || run.GameMode != GameMode.Standard)
            throw new InvalidOperationException("resumed campaign mode or saving state is invalid");
        await LiveCombatSource.OpenEnteredShopAsync();
        GD.Print("[AI-ASCENSION LIVE] standard campaign resumed through host save APIs");
    }

    private static void LoadProgress()
    {
        // Model registration must finish before progress can resolve character identities.
        var progress = SaveManager.Instance.InitProgressData();
        if (!progress.Success && progress.Status != ReadSaveStatus.FileNotFound)
            throw new InvalidOperationException("isolated campaign progress could not be loaded safely");
        // The fixture disables tutorial overlays through the ordinary host preference API.
        SaveManager.Instance.SetFtuesEnabled(false);
        GD.Print($"[AI-ASCENSION LIVE] standard progress loaded; status={progress.Status}");
    }
}
