// SPDX-License-Identifier: MIT

using System;
using System.Threading;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Connection;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Game.Lobby;
using MegaCrit.Sts2.Core.Multiplayer.Transport;
using MegaCrit.Sts2.Core.Multiplayer.Transport.ENet;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;
using MegaCrit.Sts2.Core.Nodes.Screens.MainMenu;
using MegaCrit.Sts2.Core.Platform;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class CoopNativeLobbyController
{
    private static void TickClient(ControllerState state, SceneTree tree)
    {
        if (state.Completed)
            return;
        // The first-party run handoff replaces the character-select scene. Check the owned
        // service and RunManager state before any scene-only lookup so admission can settle.
        if (TryAdmitOwnedClientRun(state))
            return;
        NSubmenuStack? stack = FindSubmenuStack(tree);
        if (stack is null)
        {
            Status = "waiting_for_main_menu_stack";
            return;
        }

        if (state.JoinScreen is null)
        {
            NJoinFriendScreen screen = stack.GetSubmenuType<NJoinFriendScreen>();
            stack.Push(screen);
            state.JoinScreen = screen;
            Status = "client_join_screen_ready";
            return;
        }

        if (state.JoinTask is null)
        {
            state.Initializer = new CapturingConnectionInitializer(
                new ENetClientConnectionInitializer(state.ClientId, state.Address, state.Port),
                service =>
                {
                    state.ClientService = service;
                    ActiveService = service;
                });
            state.JoinTask = state.JoinScreen.JoinGameAsync(state.Initializer);
            _ = state.JoinTask.ContinueWith(
                static task => _ = task.Exception,
                CancellationToken.None,
                TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
                TaskScheduler.Default);
            Status = "client_joining_via_join_flow";
            GD.Print($"[AI-ASCENSION COOP LOBBY] client joining {state.Address}:{state.Port} via NJoinFriendScreen.JoinGameAsync; host_net_id={state.HostId}");
            return;
        }

        if (!state.JoinTask.IsCompleted)
            return;
        if (state.JoinTask.IsFaulted)
        {
            Fail(state, $"JoinGameAsync failed: {state.JoinTask.Exception?.GetBaseException().Message}");
            return;
        }
        if (state.JoinTask.IsCanceled)
        {
            Fail(state, "JoinGameAsync was canceled");
            return;
        }
        if (state.ClientService is null)
        {
            Fail(state, "JoinGameAsync completed without a native client service");
            return;
        }
        if (!state.ClientService.IsConnected)
        {
            Fail(state, "JoinGameAsync completed after the native client disconnected");
            return;
        }
        if (!HasClientCharacterLobbyEvidence(stack, state))
        {
            Status = "client_waiting_for_distinct_native_identity";
            return;
        }
        if (!state.AutoAdmitRun)
        {
            Succeed(state, "client_native_lobby_joined_admission_disabled");
            return;
        }
        NCharacterSelectScreen clientScreen = stack.Peek() as NCharacterSelectScreen
            ?? throw new InvalidOperationException("client character screen disappeared");
        if (!state.ClientCharacterReady)
        {
            clientScreen.Lobby.SetLocalCharacter(ModelDb.Character<Silent>());
            clientScreen.Lobby.SetReady(true);
            state.ClientCharacterReady = true;
            Status = "client_character_ready";
            return;
        }
        GD.Print($"[AI-ASCENSION COOP LOBBY] native client joined; local_net_id={state.ClientService.NetId}; host_net_id={state.ClientService.HostNetId}");
    }
}
