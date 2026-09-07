// SPDX-License-Identifier: MIT

using System;
using System.Threading;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Connection;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Game.Lobby;
using MegaCrit.Sts2.Core.Multiplayer.Messages.Lobby;
using MegaCrit.Sts2.Core.Multiplayer.Transport;
using MegaCrit.Sts2.Core.Multiplayer.Transport.ENet;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Characters;
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;
using MegaCrit.Sts2.Core.Nodes.Screens.MainMenu;
using MegaCrit.Sts2.Core.Platform;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeCoopSessionController
{
    private static void TickClient(ControllerState state, SceneTree tree)
    {
        if (state.Completed)
            return;
        if (!state.IsRejoin && !state.JoinResultHandled && TryAdmitOwnedClientRun(state))
            return;
        if (state.RestoreTask is not null)
        {
            TickRestore(state);
            return;
        }
        if (state.JoinResultHandled)
        {
            if (!state.IsRejoin && TryAdmitOwnedClientRun(state))
                return;
            if (!state.IsRejoin && state.CharacterScreen is not null
                && state.ClientService is { IsConnected: true } connectedService)
            {
                NSubmenuStack? characterStack = FindSubmenuStack(tree);
                if (characterStack is not null)
                    CompleteClientCharacterLobby(state, characterStack, connectedService);
            }
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
            state.JoinFlow = new JoinFlow();
            state.JoinTask = state.JoinFlow.Begin(state.Initializer, tree);
            ObserveFault(state.JoinTask);
            Status = state.IsRejoin
                ? "client_rejoining_via_native_join_flow"
                : "client_joining_via_native_join_flow";
            GD.Print($"[AI-ASCENSION COOP LOBBY] client {(state.IsRejoin ? "rejoining" : "joining")} {state.Address}:{state.Port} via JoinFlow; host_net_id={state.HostId}");
            return;
        }

        if (!state.JoinTask.IsCompleted)
            return;
        if (state.JoinTask.IsFaulted)
        {
            FailJoin(state, $"JoinFlow failed: {state.JoinTask.Exception?.GetBaseException().Message}");
            return;
        }
        if (state.JoinTask.IsCanceled)
        {
            FailJoin(state, "JoinFlow was canceled");
            return;
        }
        if (state.ClientService is null && state.JoinFlow?.NetService is { } flowService)
        {
            state.ClientService = flowService;
            ActiveService = flowService;
        }
        if (state.ClientService is not { IsConnected: true } service)
        {
            FailJoin(state, "JoinFlow completed after the native client disconnected");
            return;
        }

        JoinResult result = state.JoinTask.Result;
        if (result.sessionState is not { } sessionState)
        {
            FailJoin(state, "JoinFlow completed without a session state");
            return;
        }
        switch (sessionState)
        {
            case RunSessionState.InLobby:
                if (state.IsRejoin)
                {
                    FailJoin(state, "native rejoin returned a fresh lobby");
                    return;
                }
                NSubmenuStack? lobbyStack = FindSubmenuStack(tree);
                if (lobbyStack is null)
                {
                    Status = "client_waiting_for_main_menu_stack";
                    return;
                }
                if (result.joinResponse is not { } joinResponse)
                {
                    FailJoin(state, "JoinFlow returned InLobby without a lobby response");
                    return;
                }
                state.JoinResultHandled = true;
                InitializeCharacterLobby(state, lobbyStack, service, joinResponse);
                return;

            case RunSessionState.InLoadedLobby:
                if (state.IsRejoin)
                {
                    FailJoin(state, "native rejoin returned a loaded lobby");
                    return;
                }
                NSubmenuStack? loadStack = FindSubmenuStack(tree);
                if (loadStack is null)
                {
                    Status = "client_waiting_for_main_menu_stack";
                    return;
                }
                if (result.loadJoinResponse is not { } loadResponse)
                {
                    FailJoin(state, "JoinFlow returned InLoadedLobby without a load response");
                    return;
                }
                NMultiplayerLoadGameScreen loadScreen =
                    loadStack.GetSubmenuType<NMultiplayerLoadGameScreen>();
                loadScreen.InitializeAsClient(service, loadResponse);
                loadStack.Push(loadScreen);
                state.JoinResultHandled = true;
                Succeed(state, "client_native_loaded_lobby_joined");
                return;

            case RunSessionState.Running:
                if (result.rejoinResponse is not { } rejoinResponse
                    || rejoinResponse.serializableRun is null)
                {
                    FailJoin(state, "JoinFlow returned Running without a rejoin response");
                    return;
                }
                // The response also carries NetFullCombatState, but the installed build exposes
                // no public applier. Restore the authoritative serialized run through the normal
                // multiplayer load handoff and let the native checksum/rejoin fence decide when
                // the resulting state is settled.
                state.JoinResultHandled = true;
                state.RejoinResponse = rejoinResponse;
                state.RestoreTask = RestoreRunningSession(state, service, rejoinResponse.serializableRun);
                ObserveFault(state.RestoreTask);
                Status = "client_rejoin_restore_started";
                return;

            default:
                FailJoin(state, $"JoinFlow returned unsupported session state {sessionState}");
                return;
        }
    }

    private static void InitializeCharacterLobby(
        ControllerState state,
        NSubmenuStack stack,
        NetClientGameService service,
        ClientLobbyJoinResponseMessage response)
    {
        NCharacterSelectScreen screen = stack.GetSubmenuType<NCharacterSelectScreen>();
        screen.InitializeMultiplayerAsClient(service, response);
        stack.Push(screen);
        state.CharacterScreen = screen;
        Status = "client_character_lobby_ready";
        CompleteClientCharacterLobby(state, stack, service);
    }

    private static bool HasClientCharacterLobbyEvidence(
        NSubmenuStack stack, ControllerState state)
    {
        NetClientGameService service = state.ClientService!;
        if (service.NetId == 0 || service.HostNetId == 0
            || service.HostNetId != state.HostId
            || service.NetId != state.ClientId
            || service.NetId == service.HostNetId
            || service.NetClient is not { IsConnected: true, HostNetId: not 0 })
        {
            return false;
        }

        if (stack.Peek() is not NCharacterSelectScreen characterScreen
            || characterScreen.Lobby is not { } lobby)
        {
            return false;
        }

        return ReferenceEquals(lobby.NetService, service);
    }

    private static void CompleteClientCharacterLobby(
        ControllerState state, NSubmenuStack stack, NetClientGameService service)
    {
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
        if (!state.ClientCharacterReady)
        {
            NCharacterSelectScreen screen = state.CharacterScreen
                ?? throw new InvalidOperationException("client character screen was not retained");
            screen.Lobby.SetLocalCharacter(ModelDb.Character<Silent>());
            screen.Lobby.SetReady(true);
            state.ClientCharacterReady = true;
            Status = "client_character_ready";
        }
    }

    private static void ObserveFault(Task task)
    {
        _ = task.ContinueWith(
            static completed => _ = completed.Exception,
            CancellationToken.None,
            TaskContinuationOptions.OnlyOnFaulted | TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);
    }

    private static void FailJoin(ControllerState state, string reason)
    {
        if (state.IsRejoin)
            RejoinStatus = "failed:" + reason;
        Fail(state, reason);
    }
}
