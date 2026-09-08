// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeMapV1Node(
    string Id,
    int Row,
    int Column,
    string Category,
    bool Visited);

internal sealed record RuntimeMapV1Edge(string From, string To);

internal sealed record RuntimeMapV1ActionBinding(
    string GraphNodeId,
    string HostActionId,
    string ActionNodeId);

internal sealed record RuntimeMapV1Position(string Kind, string? NodeId);

internal interface IRuntimeMapV1HostSource
{
    RuntimeMapV1Snapshot ObserveMap();
}
