// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;
using Environment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Explicit isolated combat demonstration; not a normal full-run bootstrap.</summary>
internal static class LiveCombatDemo
{
    internal static void Initialize()
    {
        if (Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT") != "1") return;
        string expected = Environment.GetEnvironmentVariable("STS2_LIVE_USER_DIR") ?? "";
        if (expected.Length == 0 || !string.Equals(Path.GetFullPath(OS.GetUserDataDir()),
            Path.GetFullPath(expected), StringComparison.OrdinalIgnoreCase))
            throw new InvalidOperationException("live demo requires its isolated user directory");
        // The game still executes real combat. Only the save backend is replaced with local
        // storage so this demonstration has no Steam cloud save writer.
        var acceptedMods = SaveManager.Instance.SettingsSave.ModSettings;
        SaveManager.MockInstanceForTesting(new SaveManager(new GodotFileIo("user://live-demo"), true));
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
        start = () =>
        {
            if (++frames < 300 || NGame.Instance == null) return;
            tree.ProcessFrame -= start;
            LiveCombatDisplay.Apply();
#if STS2_VIDEO_MENU_PROBE
            _ = VideoMenuProbe.RunAsync(tree);
#else
            _ = StartAsync();
#endif
        };
        tree.ProcessFrame += start;
        GD.Print("[AI-ASCENSION LIVE] isolated local-only save backend installed");
    }

    private static async Task StartAsync()
    {
        try
        {
            SaveManager.Instance.SetFtuesEnabled(false);
            string seed = Environment.GetEnvironmentVariable("STS2_LIVE_SEED") ?? "AIASCENSIONREPLAY1";
            if (!RuntimeV3GameplayContract.IsIdentity(seed))
                throw new InvalidOperationException("invalid replay seed");
            if (RunManager.Instance.IsInProgress)
                throw new InvalidOperationException("demo refuses to replace an active run");
            var acts = ModelDb.ActsByIndex.Select(options => options[0]).ToArray();
            await NGame.Instance!.StartNewSingleplayerRun(ModelDb.Character<Ironclad>(), false,
                acts, Array.Empty<ModifierModel>(), seed, GameMode.Custom);
            EncounterModel encounter = ModelDb.AllEncounters.Where(e => e.IsWeak
                && e.RoomType == RoomType.Monster && !e.IsDebugEncounter)
                .OrderBy(e => e.Id.ToString(), StringComparer.Ordinal).First();
            await RunManager.Instance.EnterRoomDebug(RoomType.Monster, MapPointType.Monster,
                encounter.MutableClone(), false);
            GD.Print($"[AI-ASCENSION LIVE] combat demo ready; seed={seed}; encounter={encounter.Id}");
        }
        catch (Exception error)
        {
            GD.PrintErr($"[AI-ASCENSION LIVE] bootstrap failed: {error.GetType().Name}: {error.Message}");
        }
    }
}
