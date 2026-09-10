// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeMapV1FingerprintNode(
    string Id,
    int Row,
    int Column,
    string Category,
    string TerminalKind,
    IReadOnlyList<string> ChildIds);

/// <summary>Creates a canonical public graph fingerprint from stable graph IDs.</summary>
internal static class RuntimeMapV1GraphFingerprint
{
    internal static bool TryCollectBoundedChildIds<T>(IEnumerable<T> children,
        Func<T, string?> idSelector, ref int work, ref int edgeCount,
        out List<string> childIds, out string reason)
    {
        childIds = new List<string>();
        foreach (T child in children)
        {
            if (++work > RuntimeMapV1Contract.MaxMapTraversalWork)
            {
                reason = "map_graph_work_bound_exceeded";
                return false;
            }
            if (edgeCount >= RuntimeMapV1Contract.MaxEdges)
            {
                reason = "map_edge_bound_exceeded";
                return false;
            }

            string? childId = idSelector(child);
            if (childId is null)
            {
                reason = "map_graph_edge_invalid";
                return false;
            }
            edgeCount++;
            childIds.Add(childId);
        }

        reason = string.Empty;
        return true;
    }

    internal static bool TryCreate(IReadOnlyList<RuntimeMapV1FingerprintNode> nodes,
        out string fingerprint, out string reason)
    {
        fingerprint = string.Empty;
        reason = string.Empty;
        if (nodes.Count > RuntimeMapV1Contract.MaxNodes)
        {
            reason = "map_node_bound_exceeded";
            return false;
        }

        var byId = new HashSet<string>(StringComparer.Ordinal);
        foreach (RuntimeMapV1FingerprintNode node in nodes)
        {
            if (!RuntimeMapV1Contract.IsIdentity(node.Id) || !byId.Add(node.Id))
            {
                reason = "map_graph_identity_invalid";
                return false;
            }
        }

        int edgeCount = 0;
        var builder = new StringBuilder();
        foreach (RuntimeMapV1FingerprintNode node in nodes.OrderBy(node => node.Id,
                     StringComparer.Ordinal))
        {
            if (node.ChildIds.Count > RuntimeMapV1Contract.MaxEdges)
            {
                reason = "map_edge_bound_exceeded";
                return false;
            }
            var childIds = new HashSet<string>(StringComparer.Ordinal);
            foreach (string childId in node.ChildIds)
            {
                if (!byId.Contains(childId) || !childIds.Add(childId))
                {
                    reason = "map_graph_edge_invalid";
                    return false;
                }
                if (++edgeCount > RuntimeMapV1Contract.MaxEdges)
                {
                    reason = "map_edge_bound_exceeded";
                    return false;
                }
            }

            builder.Append(node.Id).Append('|').Append(node.Row).Append(',')
                .Append(node.Column).Append(':').Append(node.Category).Append(':')
                .Append(node.TerminalKind).Append('>');
            foreach (string childId in childIds.OrderBy(id => id, StringComparer.Ordinal))
                builder.Append(childId).Append(',');
            builder.Append(';');
        }
        fingerprint = builder.ToString();
        return true;
    }
}
