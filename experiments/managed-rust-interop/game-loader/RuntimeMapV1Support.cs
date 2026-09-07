// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Host-thread composition seam for the additive read-only map profile.</summary>
internal sealed class RuntimeMapV1Support
{
    private const int Accepted = 200;
    private const int Unavailable = 503;
    private readonly IRuntimeMapV1HostSource? _source;

    private RuntimeMapV1Support(IRuntimeMapV1HostSource? source)
    {
        _source = source;
    }

    internal static RuntimeMapV1Support Unconfigured() => new(null);

    internal static RuntimeMapV1Support WithHost(IRuntimeMapV1HostSource source) =>
        new(source);

    internal string Handle(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        string leaseEpochText,
        string body,
        out int status)
    {
        status = Unavailable;
        if (_source is null)
            return RuntimeMapV1Codec.Error("map_host_unconfigured");
        string parseError = string.Empty;
        bool parsed = System.Text.Encoding.UTF8.GetByteCount(body)
            <= RuntimeMapV1Contract.MaxRequestBytes
            && (body.Length == 0 || RuntimeMapV1Codec.TryParseRequest(body, correlationId,
                instanceId, sessionId, leaseId, leaseEpochText, out _, out parseError));
        if (!parsed)
        {
            status = 400;
            return RuntimeMapV1Codec.Error(string.IsNullOrEmpty(parseError)
                ? "invalid_runtime_map_v1_request" : parseError);
        }

        RuntimeMapV1Snapshot snapshot;
        try
        {
            snapshot = _source.ObserveMap();
        }
        catch
        {
            return RuntimeMapV1Codec.Error("map_observation_unavailable");
        }
        if (!snapshot.Validate(out _))
            return RuntimeMapV1Codec.Error("invalid_runtime_map_v1_snapshot");

        if (!RuntimeMapV1Codec.TrySerializeResponse(
            correlationId, instanceId, sessionId, leaseId, ParseEpoch(leaseEpochText),
            snapshot, out string json, out _))
        {
            return RuntimeMapV1Codec.Error("invalid_runtime_map_v1_snapshot");
        }

        status = Accepted;
        return json;
    }

    private static ulong ParseEpoch(string value) =>
        ulong.TryParse(value, out ulong epoch) && epoch <= RuntimeMapV1Contract.MaxGeneration
            ? epoch : 0;
}
