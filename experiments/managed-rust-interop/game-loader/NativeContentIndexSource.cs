// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// One owner-local content-index record shaped like the Rust source input. Optional values stay
/// unknown unless the managed owner supplied that value in the same captured catalog.
/// </summary>
internal sealed record NativeContentIndexDefinition(
    string EntityKind,
    string NamespacedId,
    string? DisplayName,
    IReadOnlyList<string> Aliases,
    string? RenderedDescription,
    string? CharacterOrPool,
    string? Rarity,
    string UnlockState,
    IReadOnlyList<string> TermReferences);

/// <summary>
/// Immutable content-index source capture. ManifestId and definitions are derived from one
/// source capture and one native manifest production, so a query cannot mix catalog revisions.
/// </summary>
internal sealed record NativeContentIndexSnapshot(
    string ManifestId,
    string Locale,
    IReadOnlyList<NativeContentIndexDefinition> Definitions);

internal static partial class NativeContentCatalogManifestSource
{
    private const int MaxIndexDefinitions = 16_384;
    private const int MaxIndexTextBytes = 64 * 1024;
    private const int MaxIndexListItems = 512;

    internal static bool TryCaptureCanonicalContentIndex(
        string correlationId,
        string locale,
        out NativeContentIndexSnapshot snapshot)
    {
        snapshot = null!;
        if (!ContentManifestWireContract.ValidIdentity(correlationId)
            || !ContentManifestWireContract.ValidLocale(locale))
        {
            return false;
        }

        try
        {
            string sourceJson = CaptureJson();
            using JsonDocument sourceDocument = JsonDocument.Parse(sourceJson);
            JsonElement sourceRoot = sourceDocument.RootElement;
            if (!sourceRoot.TryGetProperty("locale", out JsonElement sourceLocale)
                || sourceLocale.GetString() != locale
                || !sourceRoot.TryGetProperty("definitions", out JsonElement sourceDefinitions)
                || sourceDefinitions.ValueKind != JsonValueKind.Array
                || sourceDefinitions.GetArrayLength() > MaxIndexDefinitions)
            {
                return false;
            }

            if (!ModEntry.TryProduceContentManifest(
                    sourceJson, correlationId, out int status, out string response)
                || status != 200
                || string.IsNullOrEmpty(response))
            {
                return false;
            }

            using JsonDocument manifestDocument = JsonDocument.Parse(response);
            JsonElement manifestRoot = manifestDocument.RootElement;
            if (!manifestRoot.TryGetProperty("manifest", out JsonElement manifest)
                || manifest.ValueKind != JsonValueKind.Object
                || ManifestField(manifest, "locale") != locale
                || ManifestField(manifest, "inventory_revision") is not string manifestId
                || !ContentManifestWireContract.ValidIdentity(manifestId)
                || !manifest.TryGetProperty("definitions", out JsonElement manifestDefinitions)
                || manifestDefinitions.ValueKind != JsonValueKind.Array)
            {
                return false;
            }

            HashSet<(string EntityKind, string NamespacedId)> manifestKeys = new();
            foreach (JsonElement definition in manifestDefinitions.EnumerateArray())
            {
                string? entityKind = ManifestField(definition, "entity_kind");
                string? namespacedId = ManifestField(definition, "namespaced_id");
                if (entityKind is null || namespacedId is null
                    || !manifestKeys.Add((entityKind, namespacedId)))
                {
                    return false;
                }
            }

            var definitions = new List<NativeContentIndexDefinition>(
                sourceDefinitions.GetArrayLength());
            foreach (JsonElement definition in sourceDefinitions.EnumerateArray())
            {
                string? entityKind = ManifestField(definition, "entity_kind");
                string? namespacedId = ManifestField(definition, "namespaced_id");
                if (entityKind is null || namespacedId is null
                    || !manifestKeys.Remove((entityKind, namespacedId)))
                {
                    return false;
                }

                string? semantic = ManifestField(definition, "semantic_inputs");
                string? localized = ManifestField(definition, "localized_text");
                definitions.Add(new NativeContentIndexDefinition(
                    entityKind,
                    namespacedId,
                    OptionalLocalizedValue(localized, "title"),
                    StringList(localized, "aliases"),
                    OptionalLocalizedValue(localized, "description"),
                    OptionalSemanticValue(semantic, "character_or_pool"),
                    OptionalSemanticValue(semantic, "rarity"),
                    OptionalSemanticValue(semantic, "unlock_state") ?? "unknown",
                    StringList(semantic, "term_references")));
            }
            if (manifestKeys.Count != 0)
                return false;

            snapshot = new NativeContentIndexSnapshot(manifestId, locale, definitions);
            return true;
        }
        catch (Exception)
        {
            snapshot = null!;
            return false;
        }
    }

    private static string? ManifestField(JsonElement value, string name) =>
        value.ValueKind == JsonValueKind.Object
            && value.TryGetProperty(name, out JsonElement field)
            && field.ValueKind == JsonValueKind.String
            ? field.GetString()
            : null;

    private static string? OptionalLocalizedValue(string? json, string name) =>
        OptionalJsonValue(json, name, static value =>
            value.ValueKind == JsonValueKind.String ? value.GetString() : null);

    private static string? OptionalSemanticValue(string? json, string name) =>
        OptionalJsonValue(json, name, static value =>
            value.ValueKind == JsonValueKind.String ? value.GetString() : null);

    private static string? OptionalJsonValue(
        string? json,
        string name,
        Func<JsonElement, string?> reader)
    {
        if (json is null)
            return null;
        if (Encoding.UTF8.GetByteCount(json) > MaxIndexTextBytes)
            throw new InvalidOperationException("content-index value exceeds its bound");
        using JsonDocument document = JsonDocument.Parse(json);
        if (document.RootElement.ValueKind != JsonValueKind.Object
            || !document.RootElement.TryGetProperty(name, out JsonElement value))
        {
            return null;
        }
        if (value.ValueKind != JsonValueKind.String)
            throw new InvalidOperationException("content-index value is malformed");
        return reader(value);
    }

    private static IReadOnlyList<string> StringList(string? json, string name)
    {
        if (json is null)
            return Array.Empty<string>();
        if (Encoding.UTF8.GetByteCount(json) > MaxIndexTextBytes)
            throw new InvalidOperationException("content-index list exceeds its bound");
        using JsonDocument document = JsonDocument.Parse(json);
        if (document.RootElement.ValueKind != JsonValueKind.Object
            || !document.RootElement.TryGetProperty(name, out JsonElement values)
            || values.ValueKind != JsonValueKind.Array)
        {
            return Array.Empty<string>();
        }
        var result = new List<string>();
        foreach (JsonElement value in values.EnumerateArray())
        {
            if (result.Count >= MaxIndexListItems)
                throw new InvalidOperationException("content-index list exceeds its bound");
            if (value.ValueKind != JsonValueKind.String
                || value.GetString() is not string text
                || text.Length == 0
                || text.IndexOfAny(['\0', '\r', '\n']) >= 0
                || Encoding.UTF8.GetByteCount(text) > MaxIndexTextBytes)
            {
                throw new InvalidOperationException("content-index list value is malformed");
            }
            result.Add(text);
        }
        return result;
    }
}
