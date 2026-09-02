// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Saves;
using MegaCrit.Sts2.Core.Timeline;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class AutoProfileUnlock
{
    private const string LogPrefix = "[AI-ASCENSION STS2 POC]";
    private const string AutoUnlockArgument = "--ai-ascension-unlock-all";
    private const int MinProfileId = 1;
    private const int MaxProfileId = 3;
    private const int MaxWaitFrames = 600;
    private static Action? _callback;
    private static bool _queued;
    private static bool _attemptRunning;
    private static bool _finished;
    private static int? _targetProfileId;
    private static int _waitFrames;

    private enum AttemptResult
    {
        NotReady,
        Applied,
        Failed
    }

    internal static void ScheduleLaunch()
    {
        bool launchArgumentRequested = HasLaunchArgument();
        if (!launchArgumentRequested)
        {
            return;
        }

        _targetProfileId = null;
        GD.Print($"{LogPrefix} automatic full unlock requested by launch argument: {AutoUnlockArgument}");
        ScheduleNextFrame();
    }

    internal static void ScheduleManualUnlock(int targetProfileId)
    {
        if (!IsValidProfileId(targetProfileId))
        {
            GD.PrintErr($"{LogPrefix} rejected unlock request for invalid profile {targetProfileId}");
            return;
        }

        if (_queued || _attemptRunning) return;
        // A manual request is a new attempt, but never interrupt an existing one.
        _finished = false;
        _waitFrames = 0;
        _targetProfileId = targetProfileId;
        GD.Print(
            $"{LogPrefix} manual full unlock requested by in-game settings; "
            + $"target=profile {targetProfileId}");
        ScheduleNextFrame();
    }

    private static bool HasLaunchArgument()
    {
        return System.Environment.GetCommandLineArgs()
            .Any(argument => string.Equals(argument, AutoUnlockArgument, StringComparison.OrdinalIgnoreCase));
    }

    private static void ScheduleNextFrame()
    {
        if (_finished || _queued || _attemptRunning)
        {
            return;
        }

        if (Engine.GetMainLoop() is not SceneTree tree)
        {
            GD.PrintErr($"{LogPrefix} could not schedule automatic full unlock: SceneTree is unavailable");
            return;
        }

        _queued = true;
        _callback = () =>
        {
            if (_callback != null)
            {
                tree.ProcessFrame -= _callback;
                _callback = null;
            }

            _queued = false;
            _attemptRunning = true;
            AttemptResult result;
            try { result = TryApply(); }
            finally { _attemptRunning = false; }

            if (result == AttemptResult.Applied || result == AttemptResult.Failed)
            {
                _finished = true;
                return;
            }

            _waitFrames++;
            if (_waitFrames >= MaxWaitFrames)
            {
                _finished = true;
                GD.PrintErr(
                    $"{LogPrefix} automatic full unlock stopped: profile data was not ready after "
                    + $"{MaxWaitFrames} frames");
                return;
            }

            if (_waitFrames % 60 == 0)
            {
                GD.Print($"{LogPrefix} waiting for profile data before automatic full unlock");
            }

            ScheduleNextFrame();
        };
        tree.ProcessFrame += _callback;
    }

    private static AttemptResult TryApply()
    {
        try
        {
            SaveManager saveManager = SaveManager.Instance;
            if (!IsProfileReady(saveManager)) return AttemptResult.NotReady;

            if (_targetProfileId is int targetProfileId)
            {
                if (!IsValidProfileId(targetProfileId))
                {
                    GD.PrintErr($"{LogPrefix} rejected unlock request for invalid profile {targetProfileId}");
                    return AttemptResult.Failed;
                }

                if (saveManager.CurrentProfileId != targetProfileId)
                {
                    saveManager.SwitchProfileId(targetProfileId);
                    GD.Print($"{LogPrefix} switched to target profile {targetProfileId}; waiting for profile data");
                    return AttemptResult.NotReady;
                }
            }

            ModelId[] cardIds = ModelDb.AllCards.Select(card => card.Id).ToArray();
            ModelId[] relicIds = ModelDb.AllRelics.Select(relic => relic.Id).ToArray();
            ModelId[] potionIds = ModelDb.AllPotions.Select(potion => potion.Id).ToArray();
            ModelId[] eventIds = ModelDb.AllEvents.Select(gameEvent => gameEvent.Id).ToArray();
            ModelId[] monsterIds = ModelDb.Monsters.Select(monster => monster.Id).ToArray();
            ModelId[] actIds = ModelDb.Acts.Select(act => act.Id).ToArray();
            ModelId[] characterIds = ModelDb.AllCharacters.Select(character => character.Id).ToArray();
            string[] epochIds = EpochModel.AllEpochIds.ToArray();

            if (cardIds.Length == 0
                || relicIds.Length == 0
                || potionIds.Length == 0
                || eventIds.Length == 0
                || monsterIds.Length == 0
                || actIds.Length == 0
                || characterIds.Length == 0
                || epochIds.Length == 0)
            {
                return AttemptResult.NotReady;
            }

            ModelId ironcladId = ModelDb.GetId<Ironclad>();
            ProgressState progress = saveManager.Progress;
            MarkDiscovered(progress, cardIds, relicIds, potionIds, eventIds, actIds);
            MarkMonsters(progress, monsterIds, ironcladId);
            RevealEpochs(saveManager, progress, epochIds);

            progress.MaxMultiplayerAscension = 10;
            foreach (ModelId characterId in characterIds)
            {
                progress.GetOrCreateCharacterStats(characterId).MaxAscension = 10;
            }

            saveManager.SaveProgressFile();
            GD.Print(
                $"{LogPrefix} automatic full unlock applied: cards={cardIds.Length}; "
                + $"relics={relicIds.Length}; potions={potionIds.Length}; events={eventIds.Length}; "
                + $"acts={actIds.Length}; monsters={monsterIds.Length}; epochs={epochIds.Length}; "
                + $"characters={characterIds.Length}; profile={saveManager.CurrentProfileId}; ascension=10");
            return AttemptResult.Applied;
        }
        catch (Exception exception)
        {
            GD.PrintErr(
                $"{LogPrefix} automatic full unlock failed: {exception.GetType().Name}");
            return AttemptResult.Failed;
        }
    }

    private static bool IsProfileReady(SaveManager saveManager)
    {
        try { return saveManager.IsProfileInitialized && saveManager.CurrentProfileId >= MinProfileId; }
        catch (InvalidOperationException) { return false; }
    }

    private static bool IsValidProfileId(int profileId) => profileId >= MinProfileId && profileId <= MaxProfileId;

    private static void MarkDiscovered(
        ProgressState progress,
        ModelId[] cardIds,
        ModelId[] relicIds,
        ModelId[] potionIds,
        ModelId[] eventIds,
        ModelId[] actIds)
    {
        foreach (ModelId cardId in cardIds)
        {
            progress.MarkCardAsSeen(cardId);
        }

        foreach (ModelId relicId in relicIds)
        {
            progress.MarkRelicAsSeen(relicId);
        }

        foreach (ModelId potionId in potionIds)
        {
            progress.MarkPotionAsSeen(potionId);
        }

        foreach (ModelId eventId in eventIds)
        {
            progress.MarkEventAsSeen(eventId);
        }

        foreach (ModelId actId in actIds)
        {
            progress.MarkActAsSeen(actId);
        }
    }

    private static void MarkMonsters(ProgressState progress, ModelId[] monsterIds, ModelId ironcladId)
    {
        foreach (ModelId monsterId in monsterIds)
        {
            EnemyStats enemyStats = progress.GetOrCreateEnemyStats(monsterId);
            if (enemyStats.FightStats.Count == 0)
            {
                enemyStats.FightStats.Add(new FightStats
                {
                    Character = ironcladId,
                    Wins = 1
                });
            }
        }
    }

    private static void RevealEpochs(SaveManager saveManager, ProgressState progress, string[] epochIds)
    {
        var revealedEpochIds = progress.Epochs
            .Where(epoch => epoch.State == EpochState.Revealed)
            .Select(epoch => epoch.Id)
            .ToHashSet(StringComparer.Ordinal);
        foreach (string epochId in epochIds)
        {
            if (!revealedEpochIds.Contains(epochId))
            {
                saveManager.ObtainEpochOverride(epochId, EpochState.Revealed);
            }
        }
    }
}
