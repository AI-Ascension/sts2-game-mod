// SPDX-License-Identifier: MIT

using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static void CheckCanonicalContentIndexSource()
    {
        NativeContentCatalogManifestSource.ControlledJson = """
            {
              "locale": "en_US",
              "definitions": [{
                "entity_kind": "ancient",
                "namespaced_id": "base:ancient:shrine",
                "semantic_inputs": "{\"epithet\":\"A\",\"rarity\":\"Rare\"}",
                "localized_text": "{\"title\":\"Shrine\",\"description\":\"A\",\"aliases\":[\"A\"]}"
              }]
            }
            """;
        ControlledManifestResponse = """
            {
              "manifest": {
                "locale": "en_US",
                "inventory_revision": "inventory:1",
                "definitions": [{
                  "entity_kind": "ancient",
                  "namespaced_id": "base:ancient:shrine"
                }]
              }
            }
            """;
        Check(
            NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndex(
                "corr-index", "en_US", out NativeContentIndexSnapshot snapshot)
            && snapshot.ManifestId == "inventory:1"
            && snapshot.Definitions.Count == 1
            && snapshot.Definitions[0].DisplayName == "Shrine"
            && snapshot.Definitions[0].RenderedDescription == "A"
            && snapshot.Definitions[0].Aliases.Count == 1
            && snapshot.Definitions[0].Rarity == "Rare",
            "managed canonical index maps localized and semantic fields from one fenced capture");

        NativeContentCatalogManifestSource.ControlledJson = """
            {
              "locale": "en_US",
              "definitions": [{
                "entity_kind": "ancient",
                "namespaced_id": "base:ancient:shrine",
                "semantic_inputs": "{\"epithet\":\"A\"}",
                "localized_text": "{\"title\":5}"
              }]
            }
            """;
        Check(
            !NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndex(
                "corr-index", "en_US", out _),
            "managed canonical index fails closed on malformed optional source values");
        NativeContentCatalogManifestSource.ControlledJson = null;
        ControlledManifestResponse = null;
    }
}
