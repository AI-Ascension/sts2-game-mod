// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class GameplayReadinessChecks
{
    internal static void Run()
    {
        RuntimeV3GameplayObservation recovery = RuntimeV3GameplayFixtures.CombatObservation(2) with
        {
            State = RuntimeV3GameplayState.Recovery,
            StateValues = new[] { "outside_combat" },
            IsActionable = false,
            InputEnabled = false,
            ModalBlocking = true
        };
        Check(!RuntimeV3GameplayReadiness.IsReady(recovery, Array.Empty<LegalActionReference>()),
            "transient recovery surface cannot establish gameplay readiness");

        RuntimeV3GameplayObservation setup = recovery with
        {
            State = RuntimeV3GameplayState.Setup,
            StateValues = new[] { "IRONCLAD" },
            IsActionable = true,
            InputEnabled = true,
            ModalBlocking = false
        };
        LegalActionReference[] startActions = new[]
        {
            new LegalActionReference("start_run:2:IRONCLAD", "start_run", "IRONCLAD", null, 2)
        };
        Check(!RuntimeV3GameplayReadiness.IsReady(setup, startActions),
            "character setup surface cannot establish post-start gameplay readiness");

        RuntimeV3GameplayObservation map = recovery with
        {
            State = RuntimeV3GameplayState.Map,
            StateValues = new[] { "map:0:0:3:Monster" },
            IsActionable = true,
            InputEnabled = true,
            ModalBlocking = false
        };
        LegalActionReference[] actions = new[]
        {
            new LegalActionReference("select_map_node:2:map:0:0:3:Monster", "select_map_node",
                "map:0:0:3:Monster", null, 2)
        };
        Check(RuntimeV3GameplayReadiness.IsReady(map, actions),
            "actionable map surface with a valid catalog establishes gameplay readiness");
        Check(!RuntimeV3GameplayReadiness.IsReady(map,
                new[] { actions[0] with { Generation = 1 } }),
            "catalog from a different generation cannot establish gameplay readiness");
        Check(!RuntimeV3GameplayReadiness.IsReady(map, Array.Empty<LegalActionReference>()),
            "empty gameplay catalog cannot establish gameplay readiness");
        Check(!RuntimeV3GameplayReadiness.IsReady(map with { ModalBlocking = true }, actions),
            "modal map surface cannot establish gameplay readiness");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
