// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class RuntimeMapV1Codec
{
    private static readonly string[] ProvenanceFields = { "artifact", "source", "generator" };
    private static readonly string[] MessageFields =
    {
        "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id",
        "session_id", "lease_id", "lease_epoch", "generation", "kind", "snapshot", "timeout"
    };

    internal static bool TrySerializeResponse(
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        RuntimeMapV1Snapshot snapshot,
        out string json,
        out string error)
    {
        json = string.Empty;
        error = string.Empty;
        if (!snapshot.Validate(out error)
            || !RuntimeMapV1Contract.IsIdentity(correlationId)
            || !RuntimeMapV1Contract.IsIdentity(instanceId)
            || !RuntimeMapV1Contract.IsIdentity(sessionId)
            || !RuntimeMapV1Contract.IsIdentity(leaseId)
            || leaseEpoch > RuntimeMapV1Contract.MaxGeneration)
        {
            error = string.IsNullOrEmpty(error) ? "map response identity is invalid" : error;
            return false;
        }

        var root = Message(
            correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            snapshot.Generation, "snapshot_response", SnapshotObject(snapshot), null);
        return TrySerialize(root, out json, out error);
    }

    internal static bool TryParseRequest(
        string body,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        string leaseEpochText,
        out ulong generation,
        out string error)
    {
        generation = 0;
        error = string.Empty;
        if (Encoding.UTF8.GetByteCount(body) > RuntimeMapV1Contract.MaxRequestBytes)
        {
            error = "map request exceeds its bound";
            return false;
        }
        try
        {
            using JsonDocument document = JsonDocument.Parse(body,
                new JsonDocumentOptions { MaxDepth = 32 });
            JsonElement root = document.RootElement;
            if (!HasExactFields(root, MessageFields)
                || StringField(root, "protocol_version") != RuntimeMapV1Contract.ProtocolVersion
                || StringField(root, "schema_digest") != RuntimeMapV1Contract.SchemaDigest
                || StringField(root, "kind") != "snapshot_request"
                || StringField(root, "correlation_id") != correlationId
                || StringField(root, "instance_id") != instanceId
                || StringField(root, "session_id") != sessionId
                || StringField(root, "lease_id") != leaseId
                || !RuntimeMapV1Contract.IsIdentity(correlationId)
                || !RuntimeMapV1Contract.IsIdentity(instanceId)
                || !RuntimeMapV1Contract.IsIdentity(sessionId)
                || !RuntimeMapV1Contract.IsIdentity(leaseId)
                || !TryEpoch(root, "lease_epoch", leaseEpochText, out _)
                || !BoundedInteger(root, "generation", out generation)
                || root.GetProperty("snapshot").ValueKind != JsonValueKind.Null
                || root.GetProperty("timeout").ValueKind != JsonValueKind.Null)
            {
                error = "invalid_runtime_map_v1_request";
                return false;
            }
            JsonElement provenance = root.GetProperty("provenance");
            if (!HasExactFields(provenance, ProvenanceFields)
                || StringField(provenance, "artifact") != RuntimeMapV1Contract.Artifact
                || StringField(provenance, "source") != RuntimeMapV1Contract.SchemaSource
                || StringField(provenance, "generator") != RuntimeMapV1Contract.Generator)
            {
                error = "invalid_runtime_map_v1_provenance";
                return false;
            }
            return true;
        }
        catch (JsonException)
        {
            error = "invalid_runtime_map_v1_json";
            return false;
        }
    }

    internal static string Error(string errorCode)
    {
        return JsonSerializer.Serialize(new Dictionary<string, string>
        {
            ["error_code"] = errorCode
        });
    }

    private static SortedDictionary<string, object?> Message(
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong generation,
        string kind,
        object? snapshot,
        object? timeout) => new()
        {
            ["protocol_version"] = RuntimeMapV1Contract.ProtocolVersion,
            ["schema_digest"] = RuntimeMapV1Contract.SchemaDigest,
            ["provenance"] = new SortedDictionary<string, string>
            {
                ["artifact"] = RuntimeMapV1Contract.Artifact,
                ["source"] = RuntimeMapV1Contract.SchemaSource,
                ["generator"] = RuntimeMapV1Contract.Generator
            },
            ["correlation_id"] = correlationId,
            ["instance_id"] = instanceId,
            ["session_id"] = sessionId,
            ["lease_id"] = leaseId,
            ["lease_epoch"] = leaseEpoch,
            ["generation"] = generation,
            ["kind"] = kind,
            ["snapshot"] = snapshot,
            ["timeout"] = timeout
        };

    private static SortedDictionary<string, object?> SnapshotObject(RuntimeMapV1Snapshot snapshot)
    {
        var nodes = new List<SortedDictionary<string, object?>>(snapshot.Nodes.Count);
        foreach (RuntimeMapV1Node node in snapshot.Nodes.OrderBy(node => node.Id,
            StringComparer.Ordinal))
        {
            nodes.Add(new SortedDictionary<string, object?>
            {
                ["id"] = node.Id, ["row"] = node.Row, ["column"] = node.Column,
                ["category"] = node.Category, ["visited"] = node.Visited
            });
        }
        var edges = new List<SortedDictionary<string, object?>>(snapshot.Edges.Count);
        foreach (RuntimeMapV1Edge edge in snapshot.Edges
            .OrderBy(edge => edge.From, StringComparer.Ordinal)
            .ThenBy(edge => edge.To, StringComparer.Ordinal))
        {
            edges.Add(new SortedDictionary<string, object?> { ["from"] = edge.From, ["to"] = edge.To });
        }
        var bindings = new List<SortedDictionary<string, object?>>(snapshot.Bindings.Count);
        foreach (RuntimeMapV1ActionBinding binding in snapshot.Bindings
            .OrderBy(binding => binding.GraphNodeId, StringComparer.Ordinal)
            .ThenBy(binding => binding.HostActionId, StringComparer.Ordinal))
        {
            bindings.Add(new SortedDictionary<string, object?>
            {
                ["graph_node_id"] = binding.GraphNodeId,
                ["host_action_id"] = binding.HostActionId,
                ["action"] = new SortedDictionary<string, object?>
                {
                    ["kind"] = "select_map_node", ["node_id"] = binding.ActionNodeId
                }
            });
        }
        object position = snapshot.Position.Kind == "current"
            ? new SortedDictionary<string, object?>
            {
                ["kind"] = snapshot.Position.Kind,
                ["node_id"] = snapshot.Position.NodeId
            }
            : new SortedDictionary<string, object?>
            {
                ["kind"] = snapshot.Position.Kind
            };
        return new SortedDictionary<string, object?>
        {
            ["state_id"] = snapshot.StateId,
            ["generation"] = snapshot.Generation,
            ["schema_version"] = snapshot.SchemaVersion,
            ["projection_version"] = snapshot.ProjectionVersion,
            ["game_build"] = snapshot.GameBuild,
            ["mod_version"] = snapshot.ModVersion,
            ["map_instance_id"] = snapshot.MapInstanceId,
            ["act_id"] = snapshot.ActId,
            ["scope_id"] = snapshot.ScopeId,
            ["availability"] = snapshot.Availability,
            ["completeness"] = snapshot.Completeness,
            ["freshness"] = snapshot.Freshness,
            ["reason"] = snapshot.Reason,
            ["nodes"] = nodes,
            ["edges"] = edges,
            ["position"] = position,
            ["history"] = snapshot.History,
            ["terminal_node_ids"] = snapshot.TerminalNodeIds.Order(StringComparer.Ordinal),
            ["bindings"] = bindings
        };
    }

    private static bool TrySerialize(SortedDictionary<string, object?> root,
        out string json, out string error)
    {
        json = JsonSerializer.Serialize(root);
        error = string.Empty;
        using JsonDocument document = JsonDocument.Parse(json);
        if (System.Text.Encoding.UTF8.GetByteCount(json) > RuntimeMapV1Contract.MaxMessageBytes
            || !PrivilegedFieldGuard.IsSafeJson(document.RootElement, out error))
        {
            json = string.Empty;
            error = string.IsNullOrEmpty(error) ? "map response exceeds its bound" : error;
            return false;
        }
        return true;
    }

    private static bool HasExactFields(JsonElement value, IReadOnlyList<string> fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var remaining = new HashSet<string>(fields, StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!remaining.Remove(property.Name)) return false;
        return remaining.Count == 0;
    }

    private static string? StringField(JsonElement value, string property) =>
        value.TryGetProperty(property, out JsonElement field) && field.ValueKind == JsonValueKind.String
            ? field.GetString() : null;

    private static bool BoundedInteger(JsonElement root, string property, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(property, out JsonElement field)
            && field.ValueKind == JsonValueKind.Number
            && field.TryGetUInt64(out value)
            && value <= RuntimeMapV1Contract.MaxGeneration;
    }

    private static bool TryEpoch(JsonElement root, string property, string expected, out ulong value)
    {
        value = 0;
        return ulong.TryParse(expected, out ulong expectedEpoch)
            && expectedEpoch <= RuntimeMapV1Contract.MaxGeneration
            && BoundedInteger(root, property, out value)
            && value == expectedEpoch;
    }
}
