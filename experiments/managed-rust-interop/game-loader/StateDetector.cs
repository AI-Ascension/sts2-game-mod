// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV3StateSignals(
    RuntimeV3GameplayState State,
    bool IsActionable,
    bool ModalBlocking,
    bool InputEnabled);

/// <summary>Source-derived state classification; unknown signals fail closed.</summary>
internal static class StateDetector
{
    internal static RuntimeV3StateSignals Detect(
        string stateName,
        bool modalBlocking,
        bool inputEnabled,
        bool hostReady)
    {
        RuntimeV3GameplayState state = stateName switch
        {
            "setup" => RuntimeV3GameplayState.Setup,
            "map" => RuntimeV3GameplayState.Map,
            "combat" => RuntimeV3GameplayState.Combat,
            "reward" => RuntimeV3GameplayState.Reward,
            "shop" => RuntimeV3GameplayState.Shop,
            "event" => RuntimeV3GameplayState.Event,
            "rest" => RuntimeV3GameplayState.Rest,
            "selection" => RuntimeV3GameplayState.Selection,
            "victory" => RuntimeV3GameplayState.Victory,
            "defeat" => RuntimeV3GameplayState.Defeat,
            "recovery" => RuntimeV3GameplayState.Recovery,
            _ => RuntimeV3GameplayState.Unknown
        };
        bool actionable = state != RuntimeV3GameplayState.Unknown
            && state != RuntimeV3GameplayState.Recovery
            && state != RuntimeV3GameplayState.Victory
            && state != RuntimeV3GameplayState.Defeat
            && hostReady
            && inputEnabled
            && !modalBlocking;
        return new RuntimeV3StateSignals(state, actionable, modalBlocking, inputEnabled);
    }
}
