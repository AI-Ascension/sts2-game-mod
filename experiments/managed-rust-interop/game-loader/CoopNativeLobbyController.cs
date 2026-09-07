// SPDX-License-Identifier: MIT

using System;
using System.Diagnostics;
using Godot;
using MegaCrit.Sts2.Core.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Connection;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
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
using RuntimeEnvironment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Bounded ordinary native ENet lobby bootstrap. This is deliberately separate from the
/// observation and protocol runtimes: it owns one host-thread attempt to start or join a lobby,
/// then leaves character selection and run admission to the game's first-party screens. Run
/// admission is disabled unless <c>STS2_NATIVE_COOP_AUTO_ADMIT_RUN</c> is explicitly enabled.
/// </summary>
internal static partial class CoopNativeLobbyController
{
    private const string RoleVariable = "STS2_NATIVE_COOP_AUTOSTART_ROLE";
    private const string PortVariable = "STS2_NATIVE_COOP_ENET_PORT";
    private const string HostIdVariable = "STS2_NATIVE_COOP_HOST_ID";
    private const string ClientIdVariable = "STS2_NATIVE_COOP_CLIENT_ID";
    private const string AutoAdmitVariable = "STS2_NATIVE_COOP_AUTO_ADMIT_RUN";
    private const string AddressVariable = "STS2_NATIVE_COOP_HOST_ADDRESS";
    private const string MaxPlayersVariable = "STS2_NATIVE_COOP_MAX_PLAYERS";
    private const int DefaultPort = 33771;
    private const int DefaultMaxPlayers = 2;
    private const int MinPort = 1024;
    private const int MaxPort = 65535;
    private const int MaxStartupSeconds = 180;
    // FindChildren's type-name filter does not match a managed C# script class on the
    // installed build. Search the live scene tree by CLR type on the game thread instead,
    // while keeping traversal bounded if a malformed scene contains an unexpectedly deep or
    // large tree.
    private const int MaxSubmenuSearchNodes = 4_096;
    private const int MaxSubmenuSearchDepth = 64;

    private static ControllerState? _state;
    private static Action? _processFrame;
    private static SceneTree? _tree;

    internal static INetGameService? ActiveService { get; private set; }
    internal static string Status { get; private set; } = "disabled";

    internal static void StartIfConfigured()
    {
        string? role = RuntimeEnvironment.GetEnvironmentVariable(RoleVariable)?.Trim().ToLowerInvariant();
        if (string.IsNullOrEmpty(role))
            return;
        if (role is not ("host" or "client"))
        {
            FailBeforeStart($"{RoleVariable} must be host or client");
            return;
        }
        if (_state is not null)
        {
            FailBeforeStart("native lobby bootstrap already started");
            return;
        }
        if (Engine.GetMainLoop() is not SceneTree tree)
        {
            FailBeforeStart("SceneTree is unavailable");
            return;
        }

        ushort port = ReadPort();
        int maxPlayers = ReadBoundedInt(MaxPlayersVariable, DefaultMaxPlayers, 2, 4);
        string address = RuntimeEnvironment.GetEnvironmentVariable(AddressVariable)?.Trim() ?? "127.0.0.1";
        if (!IsLoopbackAddress(address))
        {
            FailBeforeStart($"{AddressVariable} must be a loopback address");
            return;
        }
        ulong hostId = 0;
        ulong clientId = 0;
        if (!TryReadAutoAdmit(out bool autoAdmit, out string autoAdmitError))
        {
            FailBeforeStart(autoAdmitError);
            return;
        }
        if (role == "client" && !TryReadHostId(out hostId))
        {
            FailBeforeStart($"{HostIdVariable} is required for the client role");
            return;
        }
        if (role == "client" && !TryReadClientId(out clientId))
        {
            FailBeforeStart($"{ClientIdVariable} is required for the client role");
            return;
        }
        if (role == "client" && clientId == hostId)
        {
            FailBeforeStart($"{ClientIdVariable} must differ from {HostIdVariable}");
            return;
        }

        try
        {
            _state = new ControllerState(
                role, port, address, maxPlayers, hostId, clientId, autoAdmit);
            _tree = tree;
            _processFrame = () => Tick(tree);
            tree.ProcessFrame += _processFrame;
            Status = $"waiting_for_native_{role}_scene";
            GD.Print($"[AI-ASCENSION COOP LOBBY] configured role={role}; address={address}; port={port}; max_players={maxPlayers}; auto_admit={autoAdmit}");
        }
        catch (Exception exception)
        {
            _state?.Dispose();
            _state = null;
            _tree = null;
            _processFrame = null;
            FailBeforeStart($"bootstrap initialization failed: {exception.GetType().Name}: {exception.Message}");
        }
    }

    private static void Tick(SceneTree tree)
    {
        ControllerState? state = _state;
        if (state is null)
            return;
        state.Frames++;
        try
        {
            if (state.Role == "host")
                TickHost(state, tree);
            else
                TickClient(state, tree);
        }
        catch (Exception exception)
        {
            Fail(state, $"native lobby bootstrap threw {exception.GetType().Name}: {exception.Message}");
        }
        if (state.IsExpired() && !state.Completed)
            Fail(state, "native lobby bootstrap timed out waiting for first-party admission");
    }

    private static void TickHost(ControllerState state, SceneTree tree)
    {
        if (state.Completed)
            return;
        // RunManager changes scenes after StartRunLobby begins the run. Observe the owned
        // service/run transition before looking for the character submenu, otherwise a valid
        // admission is missed and the timeout path disconnects the newly started run.
        if (TryAdmitOwnedHostRun(state))
            return;
        NSubmenuStack? stack = FindSubmenuStack(tree);
        if (stack is null)
        {
            Status = "waiting_for_main_menu_stack";
            return;
        }

        if (state.HostService is null)
        {
            NetHostGameService service = new();
            var startError = service.StartENetHost(state.Port, state.MaxPlayers);
            if (startError.HasValue)
            {
                Fail(state, $"StartENetHost failed: {startError.Value}");
                return;
            }

            state.HostService = service;
            ActiveService = service;
            NCharacterSelectScreen screen = stack.GetSubmenuType<NCharacterSelectScreen>();
            screen.InitializeMultiplayerAsHost(service, state.MaxPlayers);
            stack.Push(screen);
            state.HostScreen = screen;
            Status = "host_character_lobby_ready";
            GD.Print($"[AI-ASCENSION COOP LOBBY] native host started on {state.Address}:{state.Port}; local_net_id={service.NetId}");
            if (!state.AutoAdmitRun)
            {
                Succeed(state, "host_native_lobby_started_admission_disabled");
                return;
            }
        }

        NCharacterSelectScreen hostScreen = state.HostScreen
            ?? throw new InvalidOperationException("host character screen was not retained");
        if (!state.AutoAdmitRun)
        {
            Succeed(state, "host_native_lobby_started_admission_disabled");
            return;
        }
        if (!state.HostCharacterConfigured)
        {
            hostScreen.Lobby.SetLocalCharacter(ModelDb.Character<Ironclad>());
            state.HostCharacterConfigured = true;
        }
        if (!state.HostCharacterReady && HasDistinctNativeParticipants(state))
        {
            hostScreen.Lobby.SetReady(true);
            state.HostCharacterReady = true;
            Status = "host_waiting_for_client_ready";
        }

        // StartRunLobby owns the all-ready check and the first-party run handoff. Setting the
        // local ready bit is the only mutation this controller performs; the lobby's own ready
        // message handler starts the run once every configured player is ready.
        if (hostScreen.Lobby.Players.Count >= state.MaxPlayers)
            Status = hostScreen.Lobby.IsAboutToBeginGame()
                ? "host_attempting_run_admission"
                : "host_waiting_for_all_ready";
    }

}
