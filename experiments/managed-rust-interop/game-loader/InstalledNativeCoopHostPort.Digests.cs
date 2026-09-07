// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private static string CreateAuthorityId(string? lobby, ulong hostNativeId) =>
        $"authority:{Sha256Hex($"native-authority|{lobby ?? "none"}|{hostNativeId:x16}")}";

    private static string CreateRunId(RunState? run, string? lobby)
    {
        string seed = run?.Rng.StringSeed ?? "lobby";
        string mode = run?.GameMode.ToString() ?? "lobby";
        return $"run:{Sha256Hex($"{mode}|{seed}|{lobby ?? "none"}")}";
    }

    private static string CreateSharedStateDigest(
        RunState? run, IReadOnlyList<ulong> nativePeerIds, bool connected)
    {
        var fields = new List<string>
        {
            connected ? "connected" : "disconnected",
            string.Join(',', nativePeerIds)
        };
        if (run is not null)
        {
            fields.Add(run.GameMode.ToString());
            fields.Add(run.Rng.StringSeed);
            fields.Add(run.CurrentActIndex.ToString(CultureInfo.InvariantCulture));
            fields.Add(run.ActFloor.ToString(CultureInfo.InvariantCulture));
            fields.Add(run.RunLocation.ToString());
            fields.Add(run.Players.Count.ToString(CultureInfo.InvariantCulture));
            foreach (Player player in run.Players.OrderBy(player => player.NetId))
            {
                fields.Add($"{player.NetId}:{player.Creature.CurrentHp}:{player.Creature.MaxHp}:{player.Gold}");
            }
        }
        return Sha256Hex(string.Join('|', fields));
    }

}
