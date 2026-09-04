// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Owns the deliberately narrow repeat-seed operation exposed by the standalone settings tab.
/// The host remains authoritative: this controller snapshots host values on the main thread,
/// asks the host to clean up the current run, and starts the replacement through NGame.
/// </summary>
internal static class SeedReplayController
{
    private const int MaxSeedLength = 128;
    private static readonly PropertyInfo? RunManagerGameModeProperty =
        typeof(RunManager).GetProperty("GameMode", BindingFlags.Instance | BindingFlags.NonPublic);
    private static bool _initialized;
    private static bool _replayQueued;
    private static bool _replayRunning;

    internal static bool CanReplayCurrentRun
    {
        get
        {
            if (!StandaloneProfileSettings.AllowRepeatingSeeds || _replayQueued || _replayRunning)
            {
                return false;
            }

            return TryCaptureSnapshot(out _, out _);
        }
    }

    internal static void Initialize()
    {
        if (_initialized)
        {
            return;
        }

        _initialized = true;
        if (!StandaloneProfileSettings.TryRegisterReplaySeedResetCallback(QueueReplay))
        {
            GD.PrintErr("[AI-ASCENSION STS2 POC] repeat-seed callback registration was not accepted");
        }
    }

    private static bool QueueReplay()
    {
        if (!StandaloneProfileSettings.AllowRepeatingSeeds || _replayQueued || _replayRunning)
        {
            return false;
        }

        if (!TryCaptureSnapshot(out ReplaySnapshot? snapshot, out string reason))
        {
            GD.PrintErr($"[AI-ASCENSION STS2 POC] repeat-seed request rejected: {reason}");
            return false;
        }

        ReplaySnapshot acceptedSnapshot = snapshot!;

        if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
        {
            GD.PrintErr("[AI-ASCENSION STS2 POC] repeat-seed request rejected: SceneTree is unavailable");
            return false;
        }

        _replayQueued = true;
        Action pump = null!;
        pump = () =>
        {
            tree.ProcessFrame -= pump;
            _replayQueued = false;
            _ = ReplayAsync(acceptedSnapshot);
        };
        tree.ProcessFrame += pump;
        GD.Print("[AI-ASCENSION STS2 POC] repeat-seed replay queued");
        return true;
    }

    private static bool TryCaptureSnapshot(out ReplaySnapshot? snapshot, out string reason)
    {
        snapshot = null;
        reason = string.Empty;

        try
        {
            RunManager runManager = RunManager.Instance;
            if (!SaveManager.Instance.IsProfileInitialized
                || SaveManager.Instance.CurrentProfileId is < 1 or > 3)
            {
                reason = "the active profile is unavailable";
                return false;
            }

            if (!runManager.IsInProgress)
            {
                reason = "no active run";
                return false;
            }

            if (runManager.NetService.Type != NetGameType.Singleplayer)
            {
                reason = "only single-player runs are supported";
                return false;
            }

            if (runManager.DailyTime != null)
            {
                reason = "daily runs are protected";
                return false;
            }

            if (!runManager.ShouldSave)
            {
                reason = "the active run has no host resume save";
                return false;
            }

            RunState? state = runManager.DebugOnlyGetState();
            if (state == null)
            {
                reason = "the host run state is unavailable";
                return false;
            }

            if (state.Players.Count != 1)
            {
                reason = "only one-player runs are supported";
                return false;
            }

            if (!TryGetGameMode(runManager, out GameMode gameMode) || gameMode != GameMode.Custom)
            {
                reason = "only custom runs are supported; standard and daily runs are protected";
                return false;
            }

            string seed = state.Rng.StringSeed.Trim();
            if (seed.Length == 0 || seed.Length > MaxSeedLength)
            {
                reason = "the host seed is missing or outside the supported length";
                return false;
            }

            snapshot = new ReplaySnapshot(
                state,
                SaveManager.Instance.CurrentProfileId,
                state.Players[0].Character,
                new List<ActModel>(state.Acts),
                new List<ModifierModel>(state.Modifiers),
                seed,
                state.AscensionLevel);
            return true;
        }
        catch (Exception exception)
        {
            reason = $"host inspection failed ({exception.GetType().Name})";
            return false;
        }
    }

    private static bool TryGetGameMode(RunManager runManager, out GameMode gameMode)
    {
        if (RunManagerGameModeProperty?.GetValue(runManager) is GameMode reflectedMode)
        {
            gameMode = reflectedMode;
            return true;
        }

        // Modifiers are not authoritative mode evidence on an unsupported host.
        gameMode = default;
        return false;
    }

    private static async Task ReplayAsync(ReplaySnapshot snapshot)
    {
        _replayRunning = true;
        bool cleanupStarted = false;
        try
        {
            RunManager runManager = RunManager.Instance;
            if (!TryCaptureSnapshot(out ReplaySnapshot? current, out string reason)
                || !StandaloneProfileSettings.AllowRepeatingSeeds
                || current == null
                || current.ProfileId != snapshot.ProfileId
                || current.SourceState != snapshot.SourceState
                || !string.Equals(current.Seed, snapshot.Seed, StringComparison.Ordinal))
            {
                string failureReason = string.IsNullOrWhiteSpace(reason) ? "the active run changed" : reason;
                GD.PrintErr($"[AI-ASCENSION STS2 POC] repeat-seed replay cancelled: {failureReason}");
                return;
            }

            if (NGame.Instance == null)
            {
                GD.PrintErr("[AI-ASCENSION STS2 POC] repeat-seed replay cancelled: NGame is unavailable");
                return;
            }

            Task? pendingSave = SaveManager.Instance.CurrentRunSaveTask;
            if (pendingSave != null)
            {
                await pendingSave;
            }

            if (!TryCaptureSnapshot(out ReplaySnapshot? savedState, out reason)
                || !StandaloneProfileSettings.AllowRepeatingSeeds
                || savedState == null
                || savedState.ProfileId != snapshot.ProfileId
                || savedState.SourceState != snapshot.SourceState
                || !string.Equals(savedState.Seed, snapshot.Seed, StringComparison.Ordinal)
                || SaveManager.Instance.CurrentRunSaveTask is { IsCompletedSuccessfully: false })
            {
                string failureReason = string.IsNullOrWhiteSpace(reason) ? "the active run changed" : reason;
                GD.PrintErr($"[AI-ASCENSION STS2 POC] repeat-seed replay cancelled after save drain: {failureReason}");
                return;
            }

            // CleanUp does not create a run-history entry. DeleteCurrentRun then removes only
            // the host-owned resume save, using the same supported APIs as the game's lifecycle.
            cleanupStarted = true;
            runManager.CleanUp(graceful: true);
            SaveManager.Instance.DeleteCurrentRun();

            await NGame.Instance.StartNewSingleplayerRun(
                character: snapshot.Character,
                shouldSave: true,
                acts: snapshot.Acts,
                modifiers: snapshot.Modifiers,
                seed: snapshot.Seed,
                gameMode: GameMode.Custom,
                ascensionLevel: snapshot.AscensionLevel,
                dailyTime: null);
            GD.Print("[AI-ASCENSION STS2 POC] repeat-seed replay started");
        }
        catch (Exception exception)
        {
            GD.PrintErr($"[AI-ASCENSION STS2 POC] repeat-seed replay failed "
                + $"{(cleanupStarted ? "after cleanup began; run recovery may be required" : "before cleanup")}: "
                + exception.GetType().Name);
        }
        finally
        {
            _replayRunning = false;
        }
    }

    private sealed record ReplaySnapshot(
        RunState SourceState,
        int ProfileId,
        CharacterModel Character,
        IReadOnlyList<ActModel> Acts,
        IReadOnlyList<ModifierModel> Modifiers,
        string Seed,
        int AscensionLevel);
}
