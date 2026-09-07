// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Managed mirror of the additive runtime-map-v1 host message. The protocol repository owns the
/// normative schema; this mirror is intentionally limited to the fields needed by the host
/// adapter and repeats its finite bounds before serialization.
/// </summary>
internal static class RuntimeMapV1Contract
{
    internal const string ProtocolVersion = "runtime-map-v1";
    internal const string Artifact = "sts2-protocol/runtime-map-v1";
    internal const string SchemaSource = "schemas/runtime-map-v1.schema.json";
    internal const string Generator = "hand-authored";
    internal const string SchemaDigest =
        "6340f3cbe6c1b5728144fe89fdfdf8645acf2f59a77c0e0c30ebfeafc77515d8";
    internal const string SnapshotSchemaVersion = "visible-map-v1";
    internal const string ProjectionVersion = "runtime-map-v1";
    internal const string ModVersion = "0.4.0";
    internal const ulong MaxGeneration = 9_007_199_254_740_991;
    internal const int MaxNodes = 256;
    internal const int MaxEdges = 1_024;
    // Traversal work is bounded independently of the output collections. This keeps a
    // malformed host graph from forcing an ever-growing queue/set before the projection can
    // report that it is unavailable.
    internal const int MaxMapTraversalWork = MaxNodes + MaxEdges + 32;
    internal const int MaxBindings = 256;
    internal const int MaxHistory = 256;
    internal const int MaxRequestBytes = 16 * 1024;
    internal const int MaxMessageBytes = 256 * 1024;
    internal const int MaxIdentityBytes = 128;
    internal const int MaxHostActionIdBytes = 512;
    internal const int MaxReasonBytes = 256;
    internal const int MaxTextBytes = 128;
    internal const int MinCoordinate = -32_768;
    internal const int MaxCoordinate = 32_767;

    internal static bool IsIdentity(string? value) =>
        !string.IsNullOrEmpty(value)
        && Encoding.UTF8.GetByteCount(value) <= MaxIdentityBytes
        && AllAscii(value, static character =>
            char.IsAsciiLetterOrDigit(character) || ".:/-_".Contains(character));

    internal static bool IsHostActionId(string? value) =>
        !string.IsNullOrEmpty(value)
        && Encoding.UTF8.GetByteCount(value) <= MaxHostActionIdBytes
        && AllAscii(value, static character =>
            char.IsAsciiLetterOrDigit(character) || ".:/-_".Contains(character));

    internal static bool IsText(string? value, int maximumBytes = MaxTextBytes)
    {
        if (string.IsNullOrEmpty(value) || Encoding.UTF8.GetByteCount(value) > maximumBytes)
        {
            return false;
        }
        foreach (char character in value)
        {
            if (char.IsControl(character)) return false;
        }
        return true;
    }

    private static bool AllAscii(string value, Func<char, bool> predicate)
    {
        foreach (char character in value)
        {
            if (character > 0x7f || !predicate(character)) return false;
        }
        return true;
    }
}

internal sealed record RuntimeMapV1Snapshot(
    string StateId,
    ulong Generation,
    string SchemaVersion,
    string ProjectionVersion,
    string GameBuild,
    string ModVersion,
    string? MapInstanceId,
    uint? ActId,
    string? ScopeId,
    string Availability,
    string Completeness,
    string Freshness,
    string? Reason,
    IReadOnlyList<RuntimeMapV1Node> Nodes,
    IReadOnlyList<RuntimeMapV1Edge> Edges,
    RuntimeMapV1Position Position,
    IReadOnlyList<string> History,
    IReadOnlyList<string> TerminalNodeIds,
    IReadOnlyList<RuntimeMapV1ActionBinding> Bindings)
{
    internal bool Validate(out string error)
    {
        error = string.Empty;
        if (Nodes is null || Edges is null || Bindings is null || History is null
            || TerminalNodeIds is null || Position is null)
        {
            error = "map snapshot contains a null collection or position";
            return false;
        }
        if (!RuntimeMapV1Contract.IsIdentity(StateId)
            || Generation > RuntimeMapV1Contract.MaxGeneration
            || SchemaVersion != RuntimeMapV1Contract.SnapshotSchemaVersion
            || !RuntimeMapV1Contract.IsIdentity(ProjectionVersion)
            || !RuntimeMapV1Contract.IsText(GameBuild)
            || !RuntimeMapV1Contract.IsText(ModVersion)
            || MapInstanceId is not null && !RuntimeMapV1Contract.IsIdentity(MapInstanceId)
            || ScopeId is not null && !RuntimeMapV1Contract.IsIdentity(ScopeId)
            || Availability is not ("available" or "unavailable" or "not_observable" or "unsupported")
            || Completeness is not ("complete" or "incomplete" or "unknown")
            || Freshness is not ("current" or "historical"))
        {
            error = "map snapshot metadata is invalid";
            return false;
        }

        bool reasonRequired = Availability != "available" || Completeness != "complete";
        if (reasonRequired != (Reason is not null)
            || Reason is not null && !RuntimeMapV1Contract.IsText(Reason, RuntimeMapV1Contract.MaxReasonBytes)
            || Availability != "available" && Completeness == "complete"
            || Availability == "available" && (MapInstanceId is null || ActId is null || ScopeId is null))
        {
            error = "map snapshot availability shape is invalid";
            return false;
        }

        if (Nodes.Count > RuntimeMapV1Contract.MaxNodes
            || Edges.Count > RuntimeMapV1Contract.MaxEdges
            || Bindings.Count > RuntimeMapV1Contract.MaxBindings
            || History.Count > RuntimeMapV1Contract.MaxHistory
            || TerminalNodeIds.Count > RuntimeMapV1Contract.MaxHistory)
        {
            error = "map snapshot collection exceeds its bound";
            return false;
        }

        var nodes = new Dictionary<string, RuntimeMapV1Node>(StringComparer.Ordinal);
        // Coordinates are player-visible projection data. Multiple rooms may occupy the same
        // screen coordinate, so preserve overlaps instead of treating them as malformed.
        foreach (RuntimeMapV1Node node in Nodes)
        {
            if (!RuntimeMapV1Contract.IsIdentity(node.Id)
                || node.Row is < RuntimeMapV1Contract.MinCoordinate or > RuntimeMapV1Contract.MaxCoordinate
                || node.Column is < RuntimeMapV1Contract.MinCoordinate or > RuntimeMapV1Contract.MaxCoordinate
                || node.Category is not ("unknown" or "start" or "monster" or "elite" or "rest"
                    or "shop" or "event" or "treasure" or "boss" or "other")
                || !nodes.TryAdd(node.Id, node))
            {
                error = "map snapshot node identity or coordinate is invalid";
                return false;
            }
        }

        var adjacency = new Dictionary<string, List<string>>(StringComparer.Ordinal);
        var edgeKeys = new HashSet<(string From, string To)>();
        foreach (RuntimeMapV1Node node in Nodes) adjacency[node.Id] = new List<string>();
        foreach (RuntimeMapV1Edge edge in Edges)
        {
            if (!nodes.ContainsKey(edge.From) || !nodes.ContainsKey(edge.To)
                || edge.From == edge.To || !edgeKeys.Add((edge.From, edge.To)))
            {
                error = "map snapshot edge is invalid";
                return false;
            }
            adjacency[edge.From].Add(edge.To);
        }

        if (HasCycle(nodes.Keys, adjacency))
        {
            error = "map snapshot graph contains a cycle";
            return false;
        }

        var history = new HashSet<string>(StringComparer.Ordinal);
        foreach (string id in History)
        {
            if (!nodes.TryGetValue(id, out RuntimeMapV1Node? historyNode)
                || !history.Add(id))
            {
                error = "map snapshot history is invalid";
                return false;
            }
            if (!historyNode.Visited)
            {
                error = "map snapshot history references an unvisited node";
                return false;
            }
        }

        var terminals = new HashSet<string>(StringComparer.Ordinal);
        foreach (string id in TerminalNodeIds)
        {
            if (!nodes.ContainsKey(id) || !terminals.Add(id))
            {
                error = "map snapshot terminal IDs are invalid";
                return false;
            }
        }

        if (Position.Kind is not ("pre_start" or "current" or "unavailable")
            || Position.Kind == "current" && (Position.NodeId is null
                || !nodes.TryGetValue(Position.NodeId, out RuntimeMapV1Node? currentNode)
                || !currentNode.Visited)
            || Position.Kind != "current" && Position.NodeId is not null
            || Position.Kind == "pre_start" && History.Count != 0)
        {
            error = "map snapshot position is invalid";
            return false;
        }

        var boundNodes = new HashSet<string>(StringComparer.Ordinal);
        var hostActions = new HashSet<string>(StringComparer.Ordinal);
        var actionOptions = new HashSet<string>(StringComparer.Ordinal);
        foreach (RuntimeMapV1ActionBinding binding in Bindings)
        {
            if (!nodes.ContainsKey(binding.GraphNodeId)
                || !RuntimeMapV1Contract.IsIdentity(binding.ActionNodeId)
                || !RuntimeMapV1Contract.IsHostActionId(binding.HostActionId)
                || binding.HostActionId == binding.GraphNodeId
                || Position.Kind == "current" && Position.NodeId == binding.GraphNodeId
                || !boundNodes.Add(binding.GraphNodeId)
                || !hostActions.Add(binding.HostActionId)
                || !actionOptions.Add(binding.ActionNodeId))
            {
                error = "map snapshot action binding is invalid";
                return false;
            }
        }
        return true;
    }

    private static bool HasCycle(IEnumerable<string> nodeIds,
        IReadOnlyDictionary<string, List<string>> adjacency)
    {
        var colors = new Dictionary<string, byte>(StringComparer.Ordinal);
        foreach (string id in nodeIds) colors[id] = 0;
        foreach (string id in nodeIds)
            if (colors[id] == 0 && Visit(id, adjacency, colors)) return true;
        return false;
    }

    private static bool Visit(string id, IReadOnlyDictionary<string, List<string>> adjacency,
        IDictionary<string, byte> colors)
    {
        colors[id] = 1;
        foreach (string child in adjacency[id])
        {
            if (colors[child] == 1 || colors[child] == 0 && Visit(child, adjacency, colors))
                return true;
        }
        colors[id] = 2;
        return false;
    }
}
