// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Emits and validates the deliberately narrow RCJ-1 action representation used by recovery.
/// This is separate from Runtime-v3 envelope serialization so a serializer change cannot alter
/// an operation digest.
/// </summary>
internal static class RuntimeV3GameplayRecoveryCanonical
{
    internal static bool TryCreate(
        LegalActionReference action, out string canonicalJson, out string payloadDigest)
    {
        canonicalJson = string.Empty;
        payloadDigest = string.Empty;
        if (!action.Validate(out _))
        {
            return false;
        }

        string nested = NestedAction(action);
        if (nested.Length == 0)
        {
            return false;
        }

        canonicalJson = $"{{\"action\":{nested},\"action_id\":\"{action.ActionId}\"}}";
        byte[] bytes = Encoding.UTF8.GetBytes(canonicalJson);
        if (bytes.Length > RuntimeV3GameplayRecoveryContract.MaxActionBytes)
        {
            canonicalJson = string.Empty;
            return false;
        }

        payloadDigest = RuntimeV3GameplayRecoveryContract.Digest(bytes);
        return true;
    }

    internal static bool TryRead(
        string canonicalJson, string expectedDigest, ulong expectedGeneration,
        out LegalActionReference? action)
    {
        action = null;
        if (string.IsNullOrEmpty(canonicalJson)
            || canonicalJson.Length > RuntimeV3GameplayRecoveryContract.MaxActionBytes
            || !RuntimeV3GameplayRecoveryContract.IsDigest(expectedDigest)
            || !TryParse(canonicalJson, out JsonDocument? document))
        {
            return false;
        }

        JsonDocument parsedDocument = document!;
        using (parsedDocument)
        {
            if (!TryReadFields(parsedDocument.RootElement, expectedGeneration,
                    out LegalActionReference? parsed)
                || parsed is null
                || !TryCreate(parsed, out string normalized, out string digest)
                || !string.Equals(normalized, canonicalJson, StringComparison.Ordinal)
                || !string.Equals(digest, expectedDigest, StringComparison.Ordinal))
            {
                return false;
            }

            action = parsed;
            return true;
        }
    }

    private static string NestedAction(LegalActionReference action)
    {
        string kind = action.Kind;
        string value = action.Value ?? string.Empty;
        string target = action.TargetId ?? string.Empty;
        return kind switch
        {
            "play_card" => $"{{\"card_id\":\"{value}\",\"kind\":\"{kind}\",\"target_id\":{NullableString(target)}}}",
            "start_run" => $"{{\"character_id\":\"{value}\",\"kind\":\"{kind}\"}}",
            "select_map_node" => $"{{\"kind\":\"{kind}\",\"node_id\":\"{value}\"}}",
            "choose_reward" => $"{{\"kind\":\"{kind}\",\"reward_id\":\"{value}\"}}",
            "shop_purchase" => $"{{\"item_id\":\"{value}\",\"kind\":\"{kind}\"}}",
            "shop_remove" or "smith" or "select_card" =>
                $"{{\"card_id\":\"{value}\",\"kind\":\"{kind}\"}}",
            "event_choice" => $"{{\"choice_id\":\"{value}\",\"kind\":\"{kind}\"}}",
            "end_turn" or "skip_reward" or "rest" or "confirm_victory" or "save_quit"
                or "proceed" or "confirm_selection" or "cancel_selection" =>
                $"{{\"kind\":\"{kind}\"}}",
            _ => string.Empty
        };
    }

    private static string NullableString(string value) => value.Length == 0 ? "null" : $"\"{value}\"";

    private static bool TryParse(string json, out JsonDocument? document)
    {
        document = null;
        try
        {
            document = JsonDocument.Parse(json, new JsonDocumentOptions
            {
                MaxDepth = 8,
                CommentHandling = JsonCommentHandling.Disallow,
                AllowTrailingCommas = false
            });
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool TryReadFields(
        JsonElement root, ulong generation, out LegalActionReference? action)
    {
        action = null;
        if (root.ValueKind != JsonValueKind.Object
            || !Exact(root, "action", "action_id")
            || !root.TryGetProperty("action_id", out JsonElement id)
            || id.ValueKind != JsonValueKind.String
            || !root.TryGetProperty("action", out JsonElement nested)
            || nested.ValueKind != JsonValueKind.Object
            || !nested.TryGetProperty("kind", out JsonElement kindElement)
            || kindElement.ValueKind != JsonValueKind.String)
        {
            return false;
        }

        string? actionId = id.GetString();
        string? kind = kindElement.GetString();
        if (actionId is null || kind is null || !RuntimeV3GameplayContract.IsIdentity(actionId)
            || !RuntimeV3GameplayContract.IsIdentity(kind))
        {
            return false;
        }

        string? value = null;
        string? target = null;
        string[] fields = kind switch
        {
            "play_card" => new[] { "card_id", "kind", "target_id" },
            "start_run" => new[] { "character_id", "kind" },
            "select_map_node" => new[] { "kind", "node_id" },
            "choose_reward" => new[] { "kind", "reward_id" },
            "shop_purchase" => new[] { "item_id", "kind" },
            "shop_remove" or "smith" or "select_card" => new[] { "card_id", "kind" },
            "event_choice" => new[] { "choice_id", "kind" },
            "end_turn" or "skip_reward" or "rest" or "confirm_victory" or "save_quit"
                or "proceed" or "confirm_selection" or "cancel_selection" => new[] { "kind" },
            _ => Array.Empty<string>()
        };
        if (fields.Length == 0 || !Exact(nested, fields))
        {
            return false;
        }

        if (kind == "play_card")
        {
            if (!TryIdentity(nested, "card_id", out value)
                || !TryNullableIdentity(nested, "target_id", out target))
            {
                return false;
            }
        }
        else if (fields.Length > 1)
        {
            string field = fields[0] == "kind" ? fields[1] : fields[0];
            if (!TryIdentity(nested, field, out value))
            {
                return false;
            }
        }

        var parsed = new LegalActionReference(actionId, kind, value, target, generation);
        if (!parsed.Validate(out _))
        {
            return false;
        }

        action = parsed;
        return true;
    }

    private static bool TryIdentity(JsonElement value, string field, out string? result)
    {
        result = null;
        return value.TryGetProperty(field, out JsonElement property)
            && property.ValueKind == JsonValueKind.String
            && RuntimeV3GameplayContract.IsIdentity(result = property.GetString() ?? string.Empty);
    }

    private static bool TryNullableIdentity(JsonElement value, string field, out string? result)
    {
        result = null;
        if (!value.TryGetProperty(field, out JsonElement property))
        {
            return false;
        }
        if (property.ValueKind == JsonValueKind.Null)
        {
            return true;
        }
        return property.ValueKind == JsonValueKind.String
            && RuntimeV3GameplayContract.IsIdentity(result = property.GetString() ?? string.Empty);
    }

    private static bool Exact(JsonElement value, params string[] fields)
    {
        var actual = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            if (!actual.Add(property.Name))
            {
                return false;
            }
        }
        if (actual.Count != fields.Length)
        {
            return false;
        }
        foreach (string field in fields)
        {
            if (!actual.Contains(field))
            {
                return false;
            }
        }
        return true;
    }
}
