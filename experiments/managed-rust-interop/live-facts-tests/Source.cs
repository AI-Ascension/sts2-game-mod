// SPDX-License-Identifier: MIT

using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

// Original controlled source shape, not a host model or copied implementation. The production
// managed bridge and the actual Rust manifest producer derive the inventory identity from it.
internal static partial class NativeContentCatalogManifestSource
{
    private static readonly string[] EntityKinds = ["card"];
    internal static string Locale = "en-US";
    internal static int Revision;
    internal static Action? OnCapture;
    internal static string CaptureJson()
    {
        OnCapture?.Invoke();
        return JsonSerializer.Serialize(new
        {
            generation_before = 1, generation_after = 1, game_build = "controlled-build",
            locale = Locale, packages = new[] { new { package_id = "base:game",
                package_version = "1", order = 0 } },
            available_entity_kinds = EntityKinds,
            registry_definition_counts = new Dictionary<string, int> { ["card"] = 1 },
            definitions = new[] { new { entity_kind = "card", namespaced_id = "strike",
                semantic_inputs = JsonSerializer.Serialize(new { revision = Revision }),
                localized_text = "{\"title\":\"Strike\"}",
                origin = new { package_id = "base:game", package_version = "1" },
                override_chain = Array.Empty<string>() } },
            adapter_compatibility = "controlled-adapter"
        });
    }
}
