// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;
using Environment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Isolated local-save bootstrap for normal model-controlled campaigns.</summary>
internal static class LiveCombatDemo
{
    internal static bool Campaign => Environment.GetEnvironmentVariable("STS2_LIVE_CAMPAIGN") == "1";
    internal static bool CampaignMapBound => Campaign
        && Environment.GetEnvironmentVariable("STS2_LIVE_CAMPAIGN_MAP_BOUND") == "1";
    internal static bool Ready { get; private set; }
    internal static RuntimeV3GameplayRunOptions RunOptions => RuntimeV3GameplayRunOptions.Parse(
        Environment.GetEnvironmentVariable("STS2_LIVE_CAMPAIGN_MODE"),
        Environment.GetEnvironmentVariable("STS2_LIVE_SEED"));

    internal static void Initialize()
    {
        if (Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT") != "1") return;
#if !STS2_COMBAT_DEMO_PROBE && !STS2_VIDEO_MENU_PROBE && !STS2_HAND_CHOICE_PROBE && !STS2_TERMINAL_PROBE
        if (!Campaign)
            throw new InvalidOperationException("the production live runtime requires campaign mode");
#endif
        string expected = Environment.GetEnvironmentVariable("STS2_LIVE_USER_DIR") ?? "";
        if (expected.Length == 0 || !string.Equals(Path.GetFullPath(OS.GetUserDataDir()),
            Path.GetFullPath(expected), StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("live demo requires its isolated user directory");
        if (!string.IsNullOrWhiteSpace(Environment.GetEnvironmentVariable(
                "STS2_SEED_PROFILE_BASELINE_IDENTITY")))
            SeededRunProfileBaseline.CaptureInitial();
        // The game still executes real combat. Only the save backend is replaced with local
        // storage so this demonstration has no Steam cloud save writer.
        var acceptedMods = SaveManager.Instance.SettingsSave.ModSettings;
        string saveRoot = Campaign && !RunOptions.Practice ? "user://live-campaign" : "user://live-demo";
        SaveManager.MockInstanceForTesting(new SaveManager(new GodotFileIo(saveRoot), true));
        SaveManager.Instance.InitProfileId(1);
        SaveManager.Instance.InitSettingsData();
        SaveManager.Instance.SettingsSave.ModSettings = acceptedMods;
        SaveManager.Instance.SettingsSave.SeenEaDisclaimer = true;
        SaveManager.Instance.SettingsSave.SkipIntroLogo = true;
        LiveCombatDisplay.ConfigureSettings();
        SaveManager.Instance.SaveSettings();
        SaveManager.Instance.InitPrefsData();
        if (Engine.GetMainLoop() is not SceneTree tree)
            throw new InvalidOperationException("demo host tree unavailable");
        int frames = 0;
        Action? start = null;
        start = async () =>
        {
            if (++frames < 300 || NGame.Instance == null) return;
            tree.ProcessFrame -= start;
            await LiveCombatDisplay.ApplyAsync();
#if STS2_VIDEO_MENU_PROBE
            _ = VideoMenuProbe.RunAsync(tree);
#else
            if (Campaign && Environment.GetEnvironmentVariable("STS2_LIVE_RESUME") == "1")
            {
                try
                {
                    if (RunOptions.Practice) throw new InvalidOperationException("practice cannot resume a standard save");
                    await LiveCampaignStart.ResumeAsync();
                }
                catch (Exception exception)
                {
                    GD.PrintErr($"[AI-ASCENSION LIVE] campaign resume failed: {exception.GetType().Name}");
                    return;
                }
            }
            Ready = true;
#if STS2_HAND_CHOICE_PROBE
            await HandChoiceProbe.PrepareAsync();
#elif STS2_TERMINAL_PROBE
            await TerminalProbe.RunAsync();
#else
#if STS2_COMBAT_DEMO_PROBE
            if (!Campaign) _ = LiveCombatFixture.StartAsync();
            else
#endif
                GD.Print("[AI-ASCENSION LIVE] campaign setup ready for model selection");
#endif
#endif
        };
        tree.ProcessFrame += start;
        GD.Print("[AI-ASCENSION LIVE] isolated local-only save backend installed");
    }

    internal static async Task<RunState> StartCampaignAsync(string characterId)
    {
        if (!Campaign || !Ready || RunManager.Instance.IsInProgress)
            throw new InvalidOperationException("campaign setup is not available");
        CharacterModel character = ModelDb.AllCharacters.SingleOrDefault(candidate =>
            string.Equals(candidate.Id.Entry, characterId, StringComparison.Ordinal))
            ?? throw new InvalidOperationException("requested character is unavailable");
        if (!LiveCombatSource.IsCampaignCharacterUnlocked(character))
            throw new InvalidOperationException("requested character is not unlocked in the native profile");
        if (!RunOptions.Practice) return await LiveCampaignStart.StandardAsync(character);
        string seed = RunOptions.Seed!;
        SaveManager.Instance.SetFtuesEnabled(false);
        var acts = ModelDb.ActsByIndex.Select(options => options[0]).ToArray();
        // Normal host setup. The model chooses the next travelable map point; no debug room entry.
        return await NGame.Instance!.StartNewSingleplayerRun(character, false,
            acts, Array.Empty<ModifierModel>(), seed, GameMode.Custom);
    }

}
