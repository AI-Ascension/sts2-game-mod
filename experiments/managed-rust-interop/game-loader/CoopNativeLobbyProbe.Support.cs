// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using Godot;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Transport;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class CoopNativeLobbyProbe
{
    private static void Stop(SceneTree tree, ProbeState state, string reason)
    {
        if (_processFrame is not null)
            tree.ProcessFrame -= _processFrame;
        _processFrame = null;
        state.WriteEvent("cleanup", new Dictionary<string, object?>
        {
            ["status"] = "probe_stopped",
            ["reason"] = reason
        }, null, null);
        state.Dispose();
        _state = null;
        GD.Print($"[AI-ASCENSION COOP PROBE] stopped: {reason}");
    }

    private static int ReadBoundedInteger(string variable, int fallback, int minimum, int maximum)
    {
        string? value = System.Environment.GetEnvironmentVariable(variable);
        return int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int parsed)
            ? Math.Clamp(parsed, minimum, maximum)
            : fallback;
    }

    private static string ReadPath(string variable, string fallback)
    {
        string? value = System.Environment.GetEnvironmentVariable(variable);
        string path = string.IsNullOrWhiteSpace(value) ? fallback : value.Trim();
        if (path.StartsWith("user://", StringComparison.Ordinal))
            throw new InvalidOperationException($"{variable} must be a filesystem path");
        return Path.GetFullPath(path);
    }

    private static string? ReadOptionalMetadata(string variable)
    {
        string? value = System.Environment.GetEnvironmentVariable(variable)?.Trim();
        return string.IsNullOrEmpty(value) ? null : value.Length <= 128 ? value : value[..128];
    }

    private static string? HashValue(string? value) => value is null ? null : Sha256Hex(value);

    private static string Sha256Hex(string value)
    {
        byte[] bytes = SHA256.HashData(Encoding.UTF8.GetBytes(value));
        return Convert.ToHexString(bytes).ToLowerInvariant();
    }

    private static string BoundedError(string message) =>
        message.Length <= 256 ? message : message[..256];

    private sealed partial class ProbeState : IDisposable
    {
        private readonly StreamWriter _publicWriter;
        private readonly StreamWriter _privateWriter;
        private readonly long _startedTimestamp;
        private readonly int _maxSeconds;
        private readonly int _intervalMilliseconds;
        private long _nextSampleTimestamp;
        private int _rows;

        internal ProbeState(
            ICoopNativeHostPort port,
            string configuredRole,
            string label,
            string outputPath,
            string privateOutputPath,
            int maxSeconds,
            int intervalMilliseconds,
            string? sourceHead,
            string? modArtifactHash,
            string? hostDllHash,
            string? hostXmlHash)
        {
            Port = port;
            ConfiguredRole = configuredRole;
            Label = label;
            _maxSeconds = maxSeconds;
            _intervalMilliseconds = intervalMilliseconds;
            _startedTimestamp = Stopwatch.GetTimestamp();
            _nextSampleTimestamp = _startedTimestamp;
            SourceHead = sourceHead;
            ModArtifactHash = modArtifactHash;
            HostDllHash = hostDllHash;
            HostXmlHash = hostXmlHash;
            Directory.CreateDirectory(Path.GetDirectoryName(outputPath) ?? ".");
            Directory.CreateDirectory(Path.GetDirectoryName(privateOutputPath) ?? ".");
            _publicWriter = new StreamWriter(outputPath, false, new UTF8Encoding(false))
            {
                AutoFlush = true
            };
            _privateWriter = new StreamWriter(privateOutputPath, false, new UTF8Encoding(false))
            {
                AutoFlush = true
            };
            WriteEvent("probe-start", new Dictionary<string, object?>
            {
                ["status"] = "started",
                ["max_seconds"] = maxSeconds,
                ["interval_milliseconds"] = intervalMilliseconds
            }, null, null);
        }

        internal ICoopNativeHostPort Port { get; }
        internal string ConfiguredRole { get; }
        internal string Label { get; }
        internal string? SourceHead { get; }
        internal string? ModArtifactHash { get; }
        internal string? HostDllHash { get; }
        internal string? HostXmlHash { get; }
        internal HashSet<ulong> PreviousConnectedPeerIds { get; set; } = new();
        internal bool HostCreatedWritten { get; set; }
        internal bool ClientJoinedWritten { get; set; }
        internal bool PostJoinWritten { get; set; }
        internal bool DisconnectWritten { get; set; }
        internal bool RejoinWritten { get; set; }

        internal bool ShouldSample()
        {
            long now = Stopwatch.GetTimestamp();
            if (now < _nextSampleTimestamp || _rows >= MaxRows)
                return false;
            _nextSampleTimestamp = now + (long)(_intervalMilliseconds / 1000.0 * Stopwatch.Frequency);
            return true;
        }

        internal bool IsExpired() =>
            Stopwatch.GetElapsedTime(_startedTimestamp).TotalSeconds >= _maxSeconds || _rows >= MaxRows;

        internal void WriteEvent(
            string eventName,
            Dictionary<string, object?>? extra,
            CoopHostObservation? observation,
            NativeTransportSnapshot? native,
            SceneRuntimeSnapshot? runtime = null)
        {
            if (_rows >= MaxRows)
                return;
            var common = new Dictionary<string, object?>(StringComparer.Ordinal)
            {
                ["probe_id"] = $"native-coop-{System.Environment.ProcessId}-{_startedTimestamp}",
                ["timestamp_utc"] = DateTimeOffset.UtcNow.ToString("O", CultureInfo.InvariantCulture),
                ["event"] = eventName,
                ["instance_label"] = Label,
                ["configured_role"] = ConfiguredRole,
                ["process_id"] = System.Environment.ProcessId,
                ["source_head"] = SourceHead,
                ["mod_artifact_sha256"] = ModArtifactHash,
                ["host_dll_sha256"] = HostDllHash,
                ["host_xml_sha256"] = HostXmlHash,
                ["source"] = "installed_native_service_and_read_first_producer"
            };
            if (extra is not null)
            {
                foreach ((string key, object? value) in extra)
                    common[key] = value;
            }
            if (observation is not null)
                common["observation"] = PublicObservation(observation);
            if (native is not null)
            {
                common["native"] = new Dictionary<string, object?>
                {
                    ["local_net_id_hash"] = HashValue(native.LocalNativeId.ToString(CultureInfo.InvariantCulture)),
                    ["host_net_id_hash"] = native.HostNativeId == 0
                        ? null : HashValue(native.HostNativeId.ToString(CultureInfo.InvariantCulture)),
                    ["connected_peer_id_hashes"] = native.ConnectedPeerIds
                        .Select(id => HashValue(id.ToString(CultureInfo.InvariantCulture))).ToArray(),
                    ["is_connected"] = native.IsConnected,
                    ["native_role"] = native.NativeRole,
                    ["platform"] = native.Platform,
                    ["lobby_id_hash"] = HashValue(native.LobbyIdentifier)
                };
            }
            if (runtime is not null)
                AddRuntimeSnapshot(common, runtime);
            string serialized = JsonSerializer.Serialize(common, JsonOptions);
            _publicWriter.WriteLine(serialized);

            if (native is not null)
            {
                var privateRow = new Dictionary<string, object?>(common, StringComparer.Ordinal)
                {
                    ["native_private"] = new Dictionary<string, object?>
                    {
                        ["local_net_id"] = native.LocalNativeId.ToString(CultureInfo.InvariantCulture),
                        ["host_net_id"] = native.HostNativeId.ToString(CultureInfo.InvariantCulture),
                        ["connected_peer_ids"] = native.ConnectedPeerIds
                            .Select(id => id.ToString(CultureInfo.InvariantCulture)).ToArray(),
                        ["lobby_identifier"] = native.LobbyIdentifier
                    }
                };
                _privateWriter.WriteLine(JsonSerializer.Serialize(privateRow, JsonOptions));
            }
            _rows++;
        }

        private static object PublicObservation(CoopHostObservation observation) => new
        {
            session_id = observation.SessionId,
            local_peer_token = observation.LocalPeerId,
            authority_id = observation.AuthorityId,
            role = observation.Role.ToString().ToLowerInvariant(),
            platform = observation.Platform,
            lobby_id_hash = HashValue(observation.LobbyIdentifier),
            host_authority_epoch = observation.AuthorityEpoch,
            run_id = observation.RunId,
            host_sequence_kind = observation.HostSequenceKind,
            host_generation = observation.HostGeneration,
            state_digest = observation.HostStateDigest,
            checkpoint_id = observation.CheckpointId,
            checksum_algorithm = observation.ChecksumAlgorithm,
            checksum_status = observation.ChecksumStatus,
            host_digest_known = observation.HostDigestKnown,
            host_loading = observation.HostLoading,
            host_divergent = observation.HostDivergent,
            recovery_required = observation.RecoveryRequired,
            peers = observation.Peers.Select(peer => new
            {
                peer_token = peer.PeerId,
                authority_id = peer.AuthorityId,
                role = peer.IsLocal ? "local" : "ally",
                connected = peer.Connected,
                peer_generation = peer.Generation,
                state_digest = peer.StateDigest,
                rejoin_epoch = peer.RejoinEpoch,
                authority_epoch = peer.AuthorityEpoch,
                checkpoint_id = peer.CheckpointId,
                digest_known = peer.DigestKnown,
                is_loading = peer.IsLoading,
                is_divergent = peer.IsDivergent,
                checksum_status = peer.ChecksumStatus
            }).ToArray()
        };

        public void Dispose()
        {
            _publicWriter.Dispose();
            _privateWriter.Dispose();
        }
    }

    private sealed record NativeTransportSnapshot(
        ulong LocalNativeId,
        ulong HostNativeId,
        bool IsConnected,
        string NativeRole,
        string Platform,
        string? LobbyIdentifier,
        ulong[] ConnectedPeerIds);
}
