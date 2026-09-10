// SPDX-License-Identifier: MIT

using System;
using System.Globalization;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static void CheckSnapshotValidation()
    {
        RuntimeMapV1Snapshot snapshot = Snapshot();
        Check(snapshot.StateId == "live:42" && snapshot.Generation == 42
            && snapshot.Validate(out _),
            "complete map uses the shared gameplay identity for its generation");
        RuntimeMapV1Snapshot unavailable = Unavailable();
        Check(unavailable.StateId == snapshot.StateId
            && unavailable.Generation == snapshot.Generation
            && unavailable.Validate(out _),
            "unavailable map preserves the shared gameplay identity");
    }

    private static void CheckCodecAndCanonicalShape()
    {
        Check(RuntimeMapV1Codec.TrySerializeResponse(
            Context.CorrelationId, Context.InstanceId, Context.SessionId, Context.LeaseId,
            ulong.Parse(Context.LeaseEpoch, CultureInfo.InvariantCulture), Snapshot(),
            out string response, out string error), $"response serializes: {error}");
        using JsonDocument document = JsonDocument.Parse(response);
        JsonElement root = document.RootElement;
        Check(root.GetProperty("schema_digest").GetString() == RuntimeMapV1Contract.SchemaDigest,
            "response carries the current schema digest");
        Check(root.GetProperty("snapshot").GetProperty("schema_version").GetString()
            == RuntimeMapV1Contract.SnapshotSchemaVersion,
            "response carries the independent visible-map schema version");
        string[] fields = root.EnumerateObject().Select(property => property.Name).ToArray();
        Check(fields.SequenceEqual(fields.OrderBy(field => field, StringComparer.Ordinal)),
            "response object keys are canonical sorted JSON");
        RuntimeMapV1Snapshot reordered = Snapshot() with
        {
            Nodes = Snapshot().Nodes.Reverse().ToArray(),
            Edges = Snapshot().Edges.Reverse().ToArray(),
            TerminalNodeIds = Snapshot().TerminalNodeIds.Reverse().ToArray(),
            Bindings = Snapshot().Bindings.Reverse().ToArray()
        };
        Check(Serialize(Snapshot()) == Serialize(reordered),
            "response collection ordering is canonical");
        using JsonDocument unavailable = JsonDocument.Parse(Serialize(Unavailable()));
        Check(unavailable.RootElement.GetProperty("snapshot").GetProperty("position")
            .EnumerateObject().Select(property => property.Name).SequenceEqual(PositionKindFields),
            "position variants omit node_id when no current node exists");
    }
}
