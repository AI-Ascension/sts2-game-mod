// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string LookupBindingProfile = "game-information-lookup-binding-v1";
    private const string LookupBindingSchemaDigest =
        "f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58";

    private static (int Status, string Response) ProcessLookupBindingWork(
        RuntimeContext context,
        string body)
    {
        if (!LookupBindingRequestIsClosed(body))
        {
            return (400, LookupBindingError(context, "malformed"));
        }

        // A success response requires a coherent, registry-backed content manifest identity.
        // The managed runtime deliberately has no fallback based on assembly bytes, paths, or
        // fixture data because those values are not a game-content manifest.
        return (503, LookupBindingError(context, "missing_capability"));
    }

    private static bool LookupBindingRequestIsClosed(string body)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(body, new JsonDocumentOptions
            {
                MaxDepth = 8
            });
            JsonElement root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
            {
                return false;
            }
            var expected = new HashSet<string>
            {
                "operation", "project_id", "run_id", "episode_id", "agent_id", "authority_epoch"
            };
            foreach (JsonProperty property in root.EnumerateObject())
            {
                if (!expected.Remove(property.Name))
                {
                    return false;
                }
            }
            return expected.Count == 0
                && root.GetProperty("operation").ValueKind == JsonValueKind.String
                && (root.GetProperty("operation").GetString() is "discovery" or "observe")
                && LookupBindingIdentity(root.GetProperty("project_id"))
                && LookupBindingIdentity(root.GetProperty("run_id"))
                && LookupBindingIdentity(root.GetProperty("episode_id"))
                && LookupBindingIdentity(root.GetProperty("agent_id"))
                && root.GetProperty("authority_epoch").TryGetUInt64(out _);
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool LookupBindingIdentity(JsonElement value) =>
        value.ValueKind == JsonValueKind.String
        && RuntimeV3GameplayContract.IsIdentity(value.GetString() ?? string.Empty);

    private static string LookupBindingError(RuntimeContext context, string code) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = LookupBindingProfile,
            ["schema_digest"] = LookupBindingSchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = "sts2-protocol/game-information-lookup-binding-v1",
                ["source"] = "schemas/game-information-lookup-binding-v1.schema.json",
                ["generator"] = "hand-authored"
            },
            ["correlation_id"] = context.CorrelationId,
            ["kind"] = "error_response",
            ["binding"] = null,
            ["discovery"] = null,
            ["observation"] = null,
            ["error"] = new Dictionary<string, object?>
            {
                ["code"] = code,
                ["field"] = null,
                ["reason"] = null
            }
        });
}
