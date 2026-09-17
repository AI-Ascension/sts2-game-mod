// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static bool TrySelectStaticEntries(
        JsonElement query,
        NativeContentIndexSnapshot snapshot,
        string scope,
        out IReadOnlyList<NativeContentIndexDefinition> entries,
        out string error)
    {
        error = string.Empty;
        string kind = query.GetProperty("query_kind").GetString()!;
        JsonElement filters = query.GetProperty("filters");
        var selected = snapshot.Definitions
            .Where(value => value.EntityKind == "card" && value.UnlockState == "unlocked")
            .Where(value => scope == "reference" || value.UnlockState == "unlocked")
            .OrderBy(value => value.EntityKind, StringComparer.Ordinal)
            .ThenBy(value => value.NamespacedId, StringComparer.Ordinal)
            .ToList();
        string? display = filters.GetProperty("display_name").ValueKind == JsonValueKind.Null
            ? null : filters.GetProperty("display_name").GetString();
        string[] ids = filters.GetProperty("namespaced_ids").EnumerateArray()
            .Select(value => value.GetString()!).ToArray();
        JsonElement[] definitionRefs = filters.GetProperty("definition_refs").EnumerateArray().ToArray();
        if (display is not null)
            selected = selected.Where(value =>
                Fold(value.DisplayName).Contains(Fold(display), StringComparison.Ordinal)
                || value.Aliases.Any(alias => Fold(alias).Contains(Fold(display), StringComparison.Ordinal))
                || (value.RenderedDescription is not null
                    && Fold(value.RenderedDescription).Contains(Fold(display), StringComparison.Ordinal)))
                .ToList();
        if (ids.Length != 0)
            selected = selected.Where(value => ids.Contains(value.NamespacedId, StringComparer.Ordinal)).ToList();
        if (definitionRefs.Length != 0)
            selected = selected.Where(value => definitionRefs.Any(reference =>
                reference.GetProperty("content_manifest_id").GetString() == snapshot.ManifestId
                && reference.GetProperty("entity_kind").GetString() == value.EntityKind
                && reference.GetProperty("namespaced_id").GetString() == value.NamespacedId)).ToList();
        if (kind is "get" or "detail")
        {
            JsonElement target = query.GetProperty("target").GetProperty("definition_ref");
            selected = selected.Where(value =>
                target.GetProperty("content_manifest_id").GetString() == snapshot.ManifestId
                && target.GetProperty("namespaced_id").GetString() == value.NamespacedId).ToList();
            if (selected.Count == 0)
            {
                bool exists = snapshot.Definitions.Any(value =>
                    value.EntityKind == "card"
                    && value.NamespacedId == target.GetProperty("namespaced_id").GetString());
                error = exists ? "denied_scope" : "unknown_id";
                entries = Array.Empty<NativeContentIndexDefinition>();
                return false;
            }
        }
        entries = selected;
        return true;
    }
}
