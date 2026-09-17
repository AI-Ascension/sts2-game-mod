// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string GameInformationCapabilities(string correlationId) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = correlationId,
            ["kind"] = "capabilities_response", ["query"] = null, ["result"] = null, ["error"] = null,
            ["capabilities"] = new Dictionary<string, object?>
            {
                ["profile"] = GameInformationProtocol, ["query_kinds"] = QueryKinds,
                ["entity_kinds"] = EntityKinds, ["projections"] = Levels, ["detail_levels"] = Levels,
                ["fields"] = Fields,
                ["limits"] = new Dictionary<string, int>
                {
                    ["item_bytes"] = 4096, ["page_bytes"] = 65536,
                    ["page_items"] = 32, ["text_bytes"] = 4096
                },
                ["max_cursor_bytes"] = MaxCursorBytes, ["max_message_bytes"] = MaxMessageBytes,
                ["snapshot_policy"] = new Dictionary<string, object?>
                {
                    ["supports_live"] = true, ["lifetime_generations"] = 1,
                    ["max_retained_snapshots"] = 1, ["expiry_behavior"] = "reject_stale_snapshot",
                    ["invalidated_by"] = InvalidatedBy
                }
            }
        });

    private static (int Status, string Response) Error(
        string correlationId, JsonElement? query, string code, string reason, int status) =>
        (status, JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = correlationId,
            ["kind"] = "error_response", ["query"] = query, ["result"] = null,
            ["capabilities"] = null,
            ["error"] = new Dictionary<string, object?>
            {
                ["code"] = code, ["field"] = null, ["reason"] = reason,
                ["retryable"] = status is 409 or 503
            }
        }));

    private static Dictionary<string, string> GameInformationProvenance() => new()
    {
        ["artifact"] = "sts2-protocol/game-information-query-v1",
        ["source"] = "schemas/game-information-query-v1.schema.json",
        ["generator"] = "hand-authored"
    };

    private static bool HasDuplicateJsonKeys(byte[] utf8)
    {
        var reader = new Utf8JsonReader(utf8, new JsonReaderOptions { MaxDepth = 24 });
        var members = new Stack<HashSet<string>>();
        while (reader.Read())
        {
            if (reader.TokenType == JsonTokenType.StartObject)
                members.Push(new HashSet<string>(StringComparer.Ordinal));
            else if (reader.TokenType == JsonTokenType.EndObject)
                members.Pop();
            else if (reader.TokenType == JsonTokenType.PropertyName
                && (members.Count == 0 || !members.Peek().Add(reader.GetString() ?? string.Empty)))
                return true;
        }
        return false;
    }
}
