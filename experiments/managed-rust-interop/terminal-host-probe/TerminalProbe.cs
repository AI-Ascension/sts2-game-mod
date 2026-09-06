// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Collections.Generic;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Commands;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.Screens;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Nodes.Screens.Overlays;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

// Explicit forced-terminal fixture. Excluded from normal builds; never gameplay evidence.
internal static class TerminalProbe
{
    internal static async Task RunAsync()
    {
        if (System.Environment.GetEnvironmentVariable("STS2_TERMINAL_PROBE") != "1"
            || !LiveCombatDemo.Campaign || !LiveCombatDemo.RunOptions.Practice
            || LiveCombatDemo.RunOptions.Seed != "AIASCENSIONTERMINALTEST1")
            throw new InvalidOperationException("terminal probe requires its isolated fixture");
        try
        {
            await LiveCombatDemo.StartCampaignAsync();
            NGame game = NGame.Instance ?? throw new InvalidOperationException("scene unavailable");
            await FramesAsync(game, 180);
            RunManager manager = RunManager.Instance;
            RunState run = await FinalActAsync(manager);
            manager.WinTime = 1;
            var source = new LiveCombatSource();
            Require(source.Observe().State != RuntimeV3GameplayState.Victory, "active run");
            manager.WinTime = 0;
            await FinishBossAsync(game, manager, run);
            GD.Print($"[AI-ASCENSION TERMINAL PROBE] room_type={run.CurrentRoom?.GetType().Name}; victory_room={run.CurrentRoom?.IsVictoryRoom}; overlay={NOverlayStack.Instance?.Peek()?.GetType().Name}");
            GD.Print($"[AI-ASCENSION TERMINAL PROBE] manager_win_time={manager.WinTime}; in_progress={manager.IsInProgress}");
            VerifyTerminal(manager, run, source);
            GD.Print("[AI-ASCENSION TERMINAL PROBE] PASS forced native terminal and stale-result checks; not a played campaign");
        }
        catch (Exception error)
        {
            GD.PrintErr($"[AI-ASCENSION TERMINAL PROBE] FAIL {error.GetType().Name}: {error.Message}");
        }
    }

    private static async Task<RunState> FinalActAsync(RunManager manager)
    {
        RunState run = manager.DebugOnlyGetState()
            ?? throw new InvalidOperationException("active run unavailable");
        await manager.SetActInternal(run.Acts.Count - 1);
        await manager.EnterAct(run.Acts.Count - 1, false);
        run = manager.DebugOnlyGetState() ?? throw new InvalidOperationException("final act unavailable");
        GD.Print($"[AI-ASCENSION TERMINAL PROBE] act_index={run.CurrentActIndex}; act_count={run.Acts.Count}");
        Require(run.CurrentActIndex == run.Acts.Count - 1, "final act selected");
        return run;
    }

    private static async Task FinishBossAsync(NGame game, RunManager manager, RunState run)
    {
        await manager.EnterMapCoordDebug(run.Map.BossMapPoint.coord, RoomType.Boss, MapPointType.Boss,
            run.Act.BossEncounter.MutableClone(), false);
        Require(run.CurrentMapCoord == run.Map.BossMapPoint.coord, "final boss map coordinate");
        await FramesAsync(game, 180);
        var combat = CombatManager.Instance.DebugOnlyGetState()
            ?? throw new InvalidOperationException("forced boss combat unavailable");
        await CreatureCmd.Kill(combat.Enemies.ToArray(), true);
        Require(combat.Enemies.All(enemy => !enemy.IsAlive), "forced enemies defeated");
        await CombatManager.Instance.EndCombatInternal();
        for (int frame = 0; frame < 600 && CombatManager.Instance.IsInProgress; frame++)
            await FramesAsync(game, 1);
        Require(!CombatManager.Instance.IsInProgress, "forced combat completed");
        await FramesAsync(game, 180);
        NMapScreen.Instance?.Close(false);
        GD.Print($"[AI-ASCENSION TERMINAL PROBE] before proceed room={run.CurrentRoom?.GetType().Name}; victory_room={run.CurrentRoom?.IsVictoryRoom}; win_time={manager.WinTime}; overlay={NOverlayStack.Instance?.Peek()?.GetType().Name}");
        var proceed = await AwaitProceedAsync(game);
        proceed?.ForceClick();
        await FramesAsync(game, 180);
    }

    private static void VerifyTerminal(RunManager manager, RunState run, LiveCombatSource source)
    {
        long fresh = manager.WinTime;
        RuntimeV3GameplayObservation terminal = source.Observe();
        GD.Print($"[AI-ASCENSION TERMINAL PROBE] manager_over={manager.IsGameOver}; run_over={run.IsGameOver}; state={terminal.State}; hp={terminal.Player.Hp}");
        Require(terminal.State == RuntimeV3GameplayState.Victory, "fresh terminal victory");
        Require(terminal.Player.Hp > 0 && !terminal.InputEnabled, "terminal player and input");
        Require(source.LegalActions(terminal).Count == 0, "no terminal mutation");
        Require(new LiveCombatSource().Observe().State != RuntimeV3GameplayState.Victory,
            "late attachment has no active lineage");
        try
        {
            manager.WinTime = 1;
            Require(source.Observe().State != RuntimeV3GameplayState.Victory, "stale win time");
            manager.WinTime = 0;
            Require(source.Observe().State != RuntimeV3GameplayState.Victory, "missing win time");
        }
        finally { manager.WinTime = fresh; }
        Require(source.Observe().State == RuntimeV3GameplayState.Victory, "fresh result restored");
    }

    private static async Task FramesAsync(NGame game, int count)
    {
        for (int frame = 0; frame < count; frame++)
            await game.ToSignal(game.GetTree(), SceneTree.SignalName.ProcessFrame);
    }

    private static void Require(bool condition, string check)
    {
        if (!condition) throw new InvalidOperationException(check);
    }

    private static async Task<NProceedButton?> AwaitProceedAsync(NGame game)
    {
        for (int frame = 0; frame < 1800; frame++)
        {
            if (Children(game).OfType<MegaCrit.Sts2.Core.Nodes.Screens.GameOverScreen.NGameOverScreen>()
                .Count(screen => screen.IsVisibleInTree()) == 1) return null;
            var buttons = Children(game).OfType<NProceedButton>()
                .Where(button => button.IsVisibleInTree() && button.IsEnabled).ToArray();
            if (buttons.Length == 1) return buttons[0];
            await game.ToSignal(game.GetTree(), SceneTree.SignalName.ProcessFrame);
        }
        string controls = string.Join(",", Children(game)
            .OfType<MegaCrit.Sts2.Core.Nodes.GodotExtensions.NButton>()
            .Where(button => button.IsVisibleInTree()).Select(button => button.GetType().Name).Distinct());
        GD.Print($"[AI-ASCENSION TERMINAL PROBE] unresolved visible control types={controls}");
        throw new InvalidOperationException("native ending control readiness timed out");
    }

    private static IEnumerable<Node> Children(Node node)
    {
        foreach (Node child in node.GetChildren())
        {
            yield return child;
            foreach (Node descendant in Children(child)) yield return descendant;
        }
    }
}
