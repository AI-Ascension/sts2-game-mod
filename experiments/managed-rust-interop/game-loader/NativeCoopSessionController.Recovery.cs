// SPDX-License-Identifier: MIT

using System;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Connection;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Game.Lobby;
using MegaCrit.Sts2.Core.Multiplayer.Messages.Lobby;
using MegaCrit.Sts2.Core.Multiplayer.Transport.ENet;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeCoopSessionController
{
    private static void TickRestore(ControllerState state)
    {
        Task restoreTask = state.RestoreTask!;
        if (!restoreTask.IsCompleted)
            return;
        state.RestoreTask = null;
        if (restoreTask.IsFaulted)
        {
            FailJoin(state, $"native running-session restore failed: {restoreTask.Exception?.GetBaseException().Message}");
            return;
        }
        if (restoreTask.IsCanceled)
        {
            FailJoin(state, "native running-session restore was canceled");
            return;
        }
        if (state.ClientService is not { IsConnected: true })
        {
            FailJoin(state, "native running-session restore completed after disconnect");
            return;
        }
        Succeed(state, state.IsRejoin
            ? "client_native_rejoin_recovered"
            : "client_native_running_session_restored");
    }

    private static async Task RestoreRunningSession(
        ControllerState state, NetClientGameService service, SerializableRun save)
    {
        RunManager manager = RunManager.Instance
            ?? throw new InvalidOperationException("run manager is unavailable for native rejoin");
        if (!ReferenceEquals(manager.NetService, service)
            && manager.DebugOnlyGetState() is not null)
        {
            // A disconnected client retains its old RunState. The supported saved-multiplayer
            // setup requires that state to be cleared before it can bind the replacement ENet
            // service and synchronizers. This branch is reached only after JoinFlow established
            // the replacement connection.
            manager.CleanUp(false);
        }

        RecoveryLobbyListener listener = new(state);
        LoadRunLobby lobby = new(service, listener, save);
        state.LoadLobby = lobby;
        try
        {
            RunState runState = RunState.FromSerializable(save);
            await manager.SetUpSavedMultiplayer(runState, lobby);
            if (NGame.Instance is not { } game)
                throw new InvalidOperationException("NGame is unavailable for native rejoin");
            await game.LoadRun(runState, save.PreFinishedRoom);
            // RunManager now owns the replacement RunLobby and the load lobby's input
            // synchronizer. Remove the temporary LoadRunLobby handlers while retaining that
            // synchronizer, matching the first-party multiplayer load screen handoff.
            lobby.CleanUp(disconnectSession: false);
            state.LoadLobby = null;
        }
        catch
        {
            try
            {
                lobby.CleanUp(disconnectSession: false);
            }
            finally
            {
                state.LoadLobby = null;
            }
            throw;
        }
    }

    /// <summary>
    /// The rejoin response contains the authoritative serialized run. LoadRunLobby still owns
    /// the first-party multiplayer synchronizers, so recovery supplies the smallest listener
    /// needed by that supported setup without pretending that a fresh character-select lobby was
    /// created.
    /// </summary>
    private sealed class RecoveryLobbyListener : ILoadRunLobbyListener
    {
        private readonly ControllerState _state;

        internal RecoveryLobbyListener(ControllerState state)
        {
            _state = state;
        }

        public void PlayerConnected(ulong playerId)
        {
        }

        public void RemotePlayerDisconnected(ulong playerId)
        {
        }

        public Task<bool> ShouldAllowRunToBegin() => Task.FromResult(true);

        public void BeginRun()
        {
        }

        public void PlayerReadyChanged(ulong playerId)
        {
        }

        public void LocalPlayerDisconnected(NetErrorInfo error)
        {
            if (_state.IsRejoin && !_state.Completed)
                RejoinStatus = "failed:native client disconnected during run restore";
        }
    }
}
