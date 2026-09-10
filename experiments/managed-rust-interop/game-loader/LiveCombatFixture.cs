// SPDX-License-Identifier: MIT

using System;
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

/// <summary>Opt-in diagnostic room fixture, excluded from production assemblies.</summary>
internal static class LiveCombatFixture
{
    internal static async Task StartAsync()
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
