// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;
using Godot;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Transport;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Bounded evidence writer for a manually created native two-instance lobby. The probe never
/// creates a lobby, sends a game message, submits an action, or invokes a rejoin API. The
/// reserved session drives the ordinary first-party UI; this class records observed state.
/// </summary>
internal static partial class CoopNativeLobbyProbe
{
    private const string RoleVariable = "STS2_NATIVE_COOP_PROBE_ROLE";
    private const string OutputVariable = "STS2_NATIVE_COOP_PROBE_OUTPUT";
    private const string PrivateOutputVariable = "STS2_NATIVE_COOP_PROBE_PRIVATE_OUTPUT";
    private const string LabelVariable = "STS2_NATIVE_COOP_PROBE_LABEL";
    private const string SourceHeadVariable = "STS2_NATIVE_COOP_PROBE_SOURCE_HEAD";
    private const string ModArtifactHashVariable = "STS2_NATIVE_COOP_PROBE_MOD_SHA256";
    private const string HostDllHashVariable = "STS2_NATIVE_COOP_PROBE_HOST_DLL_SHA256";
    private const string HostXmlHashVariable = "STS2_NATIVE_COOP_PROBE_HOST_XML_SHA256";
    private const string MaxSecondsVariable = "STS2_NATIVE_COOP_PROBE_MAX_SECONDS";
    private const string IntervalMillisecondsVariable = "STS2_NATIVE_COOP_PROBE_INTERVAL_MS";
    private const int DefaultMaxSeconds = 180;
    private const int DefaultIntervalMilliseconds = 1000;
    private const int MaxRows = 1024;
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = false };
    private static ProbeState? _state;
    private static Action? _processFrame;

    internal static void StartIfEnabled(ICoopNativeHostPort port)
    {
        string? configuredRole = System.Environment.GetEnvironmentVariable(RoleVariable);
        if (string.IsNullOrWhiteSpace(configuredRole))
            return;
        configuredRole = configuredRole.Trim().ToLowerInvariant();
        if (configuredRole is not ("host" or "client" or "auto"))
        {
            GD.PrintErr($"[AI-ASCENSION COOP PROBE] invalid {RoleVariable}: {configuredRole}");
            return;
        }

        if (_state is not null)
        {
            GD.PrintErr("[AI-ASCENSION COOP PROBE] probe is already running");
            return;
        }
        if (Engine.GetMainLoop() is not SceneTree tree)
        {
            GD.PrintErr("[AI-ASCENSION COOP PROBE] SceneTree is unavailable");
            return;
        }

        int maxSeconds = ReadBoundedInteger(MaxSecondsVariable, DefaultMaxSeconds, 5, 900);
        int intervalMilliseconds = ReadBoundedInteger(
            IntervalMillisecondsVariable, DefaultIntervalMilliseconds, 250, 10_000);
        string label = System.Environment.GetEnvironmentVariable(LabelVariable)?.Trim() ?? configuredRole;
        if (label.Length == 0 || label.Length > 64)
            label = configuredRole;

        string outputPath = ReadPath(OutputVariable,
            Path.Combine(AppContext.BaseDirectory, $"native-coop-{configuredRole}.jsonl"));
        string privateOutputPath = ReadPath(PrivateOutputVariable, outputPath + ".private.jsonl");
        try
        {
            _state = new ProbeState(
                port,
                configuredRole,
                label,
                outputPath,
                privateOutputPath,
                maxSeconds,
                intervalMilliseconds,
                ReadOptionalMetadata(SourceHeadVariable),
                ReadOptionalMetadata(ModArtifactHashVariable),
                ReadOptionalMetadata(HostDllHashVariable),
                ReadOptionalMetadata(HostXmlHashVariable));
            _processFrame = () => Tick(tree);
            tree.ProcessFrame += _processFrame;
            GD.Print($"[AI-ASCENSION COOP PROBE] recording {configuredRole} observations to {outputPath}");
        }
        catch (Exception exception)
        {
            _state?.Dispose();
            _state = null;
            _processFrame = null;
            GD.PrintErr($"[AI-ASCENSION COOP PROBE] failed to start: {exception.GetType().Name}: {exception.Message}");
        }
    }

    private static void Tick(SceneTree tree)
    {
        ProbeState? state = _state;
        if (state is null)
            return;
        state.RecordProcessFrame();
        if (state.ShouldSample())
        {
            try
            {
                RunSample(tree, state);
            }
            catch (Exception exception)
            {
                state.WriteEvent("producer-observation-error", new Dictionary<string, object?>
                {
                    ["status"] = "unknown",
                    ["error_type"] = exception.GetType().Name,
                    ["error"] = BoundedError(exception.Message)
                }, null, null);
            }
        }

        if (state.IsExpired())
            Stop(tree, state, "duration_elapsed");
    }

    private static void RunSample(SceneTree tree, ProbeState state)
    {
        RunManager manager = RunManager.Instance
            ?? throw new InvalidOperationException("run manager is unavailable");
        INetGameService service = NativeCoopSessionController.ActiveService
            ?? manager.NetService
            ?? throw new InvalidOperationException("native network service is unavailable");
        NativeTransportSnapshot native = ReadNativeTransport(service);
        CoopHostObservation observation = state.Port.Observe();

        state.WriteEvent("snapshot", null, observation, native, ReadRuntimeSnapshot(tree, state));
        string actualRole = observation.Role.ToString().ToLowerInvariant();
        if (actualRole == "host" && native.ConnectedPeerIds.Length > 1 && !state.HostCreatedWritten)
        {
            state.HostCreatedWritten = true;
            state.WriteEvent("host-created", new Dictionary<string, object?>
            {
                ["status"] = "observed_transport_peer",
                ["native_role"] = actualRole
            }, observation, native);
        }
        if (actualRole == "client" && native.IsConnected && native.HostNativeId != 0
            && !state.ClientJoinedWritten)
        {
            state.ClientJoinedWritten = true;
            state.WriteEvent("client-joined", new Dictionary<string, object?>
            {
                ["status"] = "observed_native_host_connection",
                ["native_role"] = actualRole
            }, observation, native);
        }

        bool hasTwoPeerTransport = native.IsConnected && native.ConnectedPeerIds.Length >= 2;
        if (hasTwoPeerTransport && !state.PostJoinWritten)
        {
            state.PostJoinWritten = true;
            state.WriteEvent("post-join-convergence", new Dictionary<string, object?>
            {
                ["status"] = "not_proven",
                ["reason"] = "producer_digest_unknown_or_checksum_unread",
                ["connected_peer_count"] = native.ConnectedPeerIds.Length
            }, observation, native);
            state.WriteEvent("action-or-vote", new Dictionary<string, object?>
            {
                ["status"] = "not_attempted",
                ["reason"] = "native_synchronizer_not_admitted"
            }, observation, native);
        }

        bool remoteWasConnected = state.PreviousConnectedPeerIds.Count > 1;
        bool remoteIsConnected = native.ConnectedPeerIds.Length > 1;
        if (remoteWasConnected && !remoteIsConnected && !state.DisconnectWritten)
        {
            state.DisconnectWritten = true;
            state.WriteEvent("disconnect", new Dictionary<string, object?>
            {
                ["status"] = "observed_transport_loss"
            }, observation, native);
            state.WriteEvent("rejoin-attempt", new Dictionary<string, object?>
            {
                ["status"] = "not_instrumented",
                ["reason"] = "use_the_normal_native_run_lobby_path_once"
            }, observation, native);
        }
        if (state.DisconnectWritten && !remoteWasConnected && remoteIsConnected && !state.RejoinWritten)
        {
            state.RejoinWritten = true;
            state.WriteEvent("post-rejoin-observation", new Dictionary<string, object?>
            {
                ["status"] = "observed_transport_return",
                ["convergence"] = "not_proven"
            }, observation, native);
        }
        state.PreviousConnectedPeerIds = native.ConnectedPeerIds.ToHashSet();
    }

    private static NativeTransportSnapshot ReadNativeTransport(INetGameService service)
    {
        var connected = new HashSet<ulong>();
        ulong hostId = 0;
        bool nativeConnected = service.IsConnected;
        if (service is INetHostGameService host && host.NetHost is { } nativeHost)
        {
            nativeConnected &= nativeHost.IsConnected;
            if (nativeConnected)
            {
                connected.Add(nativeHost.NetId);
                foreach (ulong peerId in nativeHost.ConnectedPeerIds)
                    connected.Add(peerId);
            }
            hostId = nativeHost.NetId;
        }
        else if (service is INetClientGameService client && client.NetClient is { } nativeClient)
        {
            nativeConnected &= nativeClient.IsConnected;
            hostId = nativeClient.HostNetId;
            if (nativeConnected)
            {
                connected.Add(nativeClient.NetId);
                if (hostId != 0)
                    connected.Add(hostId);
            }
        }
        else if (nativeConnected)
        {
            connected.Add(service.NetId);
        }

        return new NativeTransportSnapshot(
            service.NetId,
            hostId,
            nativeConnected,
            service.Type.ToString().ToLowerInvariant(),
            service.Platform.ToString().ToLowerInvariant(),
            service.GetRawLobbyIdentifier(),
            connected.OrderBy(id => id).ToArray());
    }
}
