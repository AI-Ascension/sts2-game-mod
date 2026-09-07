// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
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
using MegaCrit.Sts2.Core.Nodes.Screens.CharacterSelect;
using MegaCrit.Sts2.Core.Nodes.Screens.MainMenu;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeCoopSessionController
{
    private static NSubmenuStack? FindSubmenuStack(SceneTree tree)
    {
        if (tree.Root is null)
            return null;
        var pending = new Stack<(Node Node, int Depth)>();
        pending.Push((tree.Root, 0));
        int visited = 0;
        while (pending.Count > 0 && visited < MaxSubmenuSearchNodes)
        {
            (Node node, int depth) = pending.Pop();
            visited++;
            if (node is NSubmenuStack stack)
                return stack;
            if (depth >= MaxSubmenuSearchDepth)
                continue;
            foreach (Node child in node.GetChildren())
                pending.Push((child, depth + 1));
        }
        return null;
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

    private static bool TryAdmitOwnedHostRun(ControllerState state)
    {
        RunManager? manager = RunManager.Instance;
        if (manager is null || state.HostService is not { IsConnected: true } service
            || !ReferenceEquals(manager.NetService, service))
        {
            return false;
        }

        return CoopNativeLobbyAdmission.TryAdmitOwnedRun(
            state.AutoAdmitRun,
            manager.IsInProgress,
            service.IsConnected,
            true,
            () =>
            {
                Succeed(state, "host_run_admitted");
                GD.Print("[AI-ASCENSION COOP LOBBY] first-party native run admission observed on host");
            });
    }

    private static bool TryAdmitOwnedClientRun(ControllerState state)
    {
        RunManager? manager = RunManager.Instance;
        if (manager is null || state.ClientService is not { IsConnected: true } service
            || !ReferenceEquals(manager.NetService, service))
        {
            return false;
        }

        return CoopNativeLobbyAdmission.TryAdmitOwnedRun(
            state.AutoAdmitRun,
            manager.IsInProgress,
            service.IsConnected,
            true,
            () =>
            {
                Succeed(state, "client_run_admitted");
                GD.Print("[AI-ASCENSION COOP LOBBY] first-party native run admission observed on client");
            });
    }

    private static bool HasDistinctNativeParticipants(ControllerState state)
    {
        NetHostGameService? service = state.HostService;
        if (service is null || !service.IsConnected || service.NetId == 0
            || service.NetHost is not { IsConnected: true } host)
        {
            return false;
        }
        var participants = new HashSet<ulong> { service.NetId };
        foreach (ulong peerId in host.ConnectedPeerIds)
        {
            if (peerId != 0)
                participants.Add(peerId);
        }
        return participants.Count >= state.MaxPlayers;
    }

    private static void FailBeforeStart(string reason)
    {
        Status = "failed:" + reason;
        GD.PrintErr($"[AI-ASCENSION COOP LOBBY] {Status}");
    }

    private static void Succeed(ControllerState state, string status)
    {
        state.Completed = true;
        Status = status;
        if (state.IsRejoin)
            RejoinStatus = "recovered";
        DetachProcessFrame();
    }

    private static void Fail(ControllerState state, string reason)
    {
        state.Completed = true;
        Status = "failed:" + reason;
        if (state.IsRejoin)
            RejoinStatus = Status;
        ActiveService = null;
        DetachProcessFrame();
        state.Dispose();
        GD.PrintErr($"[AI-ASCENSION COOP LOBBY] {Status}");
    }

    private static void DetachProcessFrame()
    {
        if (_tree is not null && _processFrame is not null)
            _tree.ProcessFrame -= _processFrame;
        _tree = null;
        _processFrame = null;
    }

    private sealed class ControllerState : IDisposable
    {
        internal ControllerState(string role, ushort port, string address, int maxPlayers,
            ulong hostId, ulong clientId, bool autoAdmitRun)
        {
            Role = role;
            Port = port;
            Address = address;
            MaxPlayers = maxPlayers;
            HostId = hostId;
            ClientId = clientId;
            AutoAdmitRun = autoAdmitRun;
            DeadlineTimestamp = Stopwatch.GetTimestamp()
                + (long)(MaxStartupSeconds * Stopwatch.Frequency);
        }

        internal string Role { get; }
        internal ushort Port { get; }
        internal string Address { get; }
        internal int MaxPlayers { get; }
        internal ulong HostId { get; }
        internal ulong ClientId { get; }
        internal bool AutoAdmitRun { get; }
        internal long DeadlineTimestamp { get; }
        internal int Frames { get; set; }
        internal bool Completed { get; set; }
        internal NetHostGameService? HostService { get; set; }
        internal NetClientGameService? ClientService { get; set; }
        internal NCharacterSelectScreen? HostScreen { get; set; }
        internal bool HostCharacterConfigured { get; set; }
        internal bool HostCharacterReady { get; set; }
        internal bool ClientCharacterReady { get; set; }
        internal CapturingConnectionInitializer? Initializer { get; set; }
        internal JoinFlow? JoinFlow { get; set; }
        internal Task<JoinResult>? JoinTask { get; set; }
        internal NCharacterSelectScreen? CharacterScreen { get; set; }
        internal LoadRunLobby? LoadLobby { get; set; }
        internal ClientRejoinResponseMessage? RejoinResponse { get; set; }
        internal Task? RestoreTask { get; set; }
        internal bool JoinResultHandled { get; set; }
        internal bool IsRejoin { get; set; }
        private bool _disposed;

        internal bool IsExpired() => Stopwatch.GetTimestamp() >= DeadlineTimestamp;

        public void Dispose()
        {
            if (_disposed)
                return;
            _disposed = true;
            TryCleanup("join cancellation", static state => state.Initializer?.Cancel(), this);
            TryCleanup("join flow cancellation", static state => state.JoinFlow?.CancelToken.Cancel(), this);
            TryCleanup("load lobby cleanup", static state =>
            {
                state.LoadLobby?.CleanUp(disconnectSession: false);
                state.LoadLobby = null;
            }, this);
            TryCleanup("client disconnect", static state =>
                state.ClientService?.Disconnect(NetError.CancelledJoin, true), this);
            TryCleanup("host disconnect", static state =>
                state.HostService?.Disconnect(NetError.CancelledJoin, true), this);
            TryCleanup("initializer dispose", static state => state.Initializer?.Dispose(), this);
        }

        private static void TryCleanup(
            string operation, Action<ControllerState> cleanup, ControllerState state)
        {
            try
            {
                cleanup(state);
            }
            catch (Exception exception)
            {
                GD.PrintErr(
                    $"[AI-ASCENSION COOP LOBBY] cleanup {operation} failed: {exception.GetType().Name}");
            }
        }
    }

    /// <summary>Delegates to the first-party ENet initializer while retaining the service that
    /// JoinFlow creates internally. JoinFlow owns that service; constructing a second client here
    /// would observe a disconnected transport and bypass the normal join flow.</summary>
    private sealed class CapturingConnectionInitializer : IClientConnectionInitializer, IDisposable
    {
        private readonly ENetClientConnectionInitializer _inner;
        private readonly Action<NetClientGameService> _capture;
        private readonly CancellationTokenSource _cancel = new();

        internal CapturingConnectionInitializer(
            ENetClientConnectionInitializer inner,
            Action<NetClientGameService> capture)
        {
            _inner = inner;
            _capture = capture;
        }

        public async Task<NetErrorInfo?> Connect(
            NetClientGameService gameService,
            CancellationToken cancelToken)
        {
            _capture(gameService);
            using CancellationTokenSource linked = CancellationTokenSource.CreateLinkedTokenSource(
                cancelToken, _cancel.Token);
            return await _inner.Connect(gameService, linked.Token);
        }

        internal void Cancel()
        {
            _cancel.Cancel();
        }

        public void Dispose()
        {
            _cancel.Dispose();
        }
    }
}
