// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Nodes.Screens.GameOverScreen;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private RunState? _observedRun;
    private long _winTimeBeforeTerminal;

    private static Player? TerminalPlayer()
    {
        RunManager manager = RunManager.Instance;
        RunState? run = manager.DebugOnlyGetState();
        bool terminal = manager.IsGameOver || manager.WinTime > 0 && HasVictorySurface();
        if (!terminal || run == null || run.Players.Count != 1
            || manager.NetService.Type != NetGameType.Singleplayer) return null;
        return LocalContext.GetMe(run);
    }

    private RuntimeV3GameplayObservation ProjectVictory(RuntimeV3GameplayObservation observation)
    {
        RunManager manager = RunManager.Instance;
        RunState? run = manager.DebugOnlyGetState();
        if (run == null) return observation;
        if (!ReferenceEquals(_observedRun, run) && manager.IsInProgress && !run.IsGameOver
            && !HasVictorySurface())
        {
            _observedRun = run;
            _winTimeBeforeTerminal = manager.WinTime;
        }
        if (!ReferenceEquals(_observedRun, run)
            || manager.IsAbandoned || manager.WinTime <= 0
            || manager.WinTime == _winTimeBeforeTerminal || TerminalPlayer()?.Creature.IsAlive != true)
            return observation;
        if (!HasVictorySurface()) return observation;
        return observation with
        {
            State = RuntimeV3GameplayState.Victory, StateValues = Array.Empty<string>(),
            IsActionable = false, InputEnabled = false, ModalBlocking = false
        };
    }

    private static bool HasVictorySurface()
    {
        if (NGame.Instance is not { } game) return false;
        if (Descendants(game).OfType<NGameOverScreen>().Count(screen => screen.IsVisibleInTree()) == 1)
            return true;
        return RunManager.Instance.DebugOnlyGetState()?.CurrentRoom?.IsVictoryRoom == true
            && NEventRoom.Instance?.IsVisibleInTree() == true
            && NMapScreen.Instance?.IsOpen != true && RewardOverlay() == null;
    }
}
