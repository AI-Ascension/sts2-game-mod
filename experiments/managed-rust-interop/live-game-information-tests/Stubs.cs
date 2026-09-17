// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
namespace AiAscension.Sts2GameMod.Runtime;
internal static class ContentManifestWireContract { internal static bool ValidIdentity(string value) => !string.IsNullOrEmpty(value); internal static bool ValidLocale(string value) => !string.IsNullOrEmpty(value); }
internal sealed record NativeContentIndexDefinition(
    string EntityKind,
    string NamespacedId,
    string? DisplayName,
    IReadOnlyList<string> Aliases,
    string? RenderedDescription,
    string? CharacterOrPool,
    string? Rarity,
    string UnlockState,
    IReadOnlyList<string> TermReferences,
    IReadOnlyList<string>? Tags);
internal sealed record NativeContentIndexSnapshot(
    string ManifestId,
    string Locale,
    IReadOnlyList<NativeContentIndexDefinition> Definitions);
internal sealed record NativeContentIndexCapture(
    NativeContentIndexSnapshot Snapshot,
    string SourceJson);
internal static partial class NativeContentCatalogManifestSource
{
    private static readonly string[] ProbeEntityKinds = ["card", "relic"];
    internal static NativeContentIndexSnapshot? ControlledSnapshot { get; set; }
    internal static bool TryCaptureCanonicalContentIndex(string _, string __, out NativeContentIndexSnapshot snapshot)
    {
        snapshot = ControlledSnapshot!;
        return snapshot is not null;
    }
    internal static bool TryCaptureCanonicalContentIndexWithSource(string _, string __, out NativeContentIndexCapture capture)
    {
        if (ControlledSnapshot is null)
        {
            capture = null!;
            return false;
        }
        var definitions = new List<object>();
        foreach (NativeContentIndexDefinition definition in ControlledSnapshot.Definitions)
        {
            var semantic = new Dictionary<string, object?>();
            if (definition.CharacterOrPool is not null)
                semantic["character_or_pool"] = definition.CharacterOrPool;
            if (definition.Rarity is not null)
                semantic["rarity"] = definition.Rarity;
            semantic["unlock_state"] = definition.UnlockState;
            if (definition.TermReferences.Count != 0)
                semantic["term_references"] = definition.TermReferences;
            if (definition.Tags is not null)
                semantic["tags"] = definition.Tags;
            var localized = new Dictionary<string, object?>();
            if (definition.DisplayName is not null)
                localized["title"] = definition.DisplayName;
            if (definition.Aliases.Count != 0)
                localized["aliases"] = definition.Aliases;
            if (definition.RenderedDescription is not null)
                localized["description"] = definition.RenderedDescription;
            definitions.Add(new
            {
                entity_kind = definition.EntityKind,
                namespaced_id = definition.NamespacedId,
                semantic_inputs = JsonSerializer.Serialize(semantic),
                localized_text = JsonSerializer.Serialize(localized),
                origin = new { package_id = "base", package_version = "1" },
                override_chain = Array.Empty<string>()
            });
        }
        string sourceJson = JsonSerializer.Serialize(new
        {
            generation_before = 7,
            generation_after = 7,
            game_build = "build:probe",
            locale = ControlledSnapshot.Locale,
            packages = new[] { new { package_id = "base", package_version = "1", order = 0 } },
            available_entity_kinds = ProbeEntityKinds,
            registry_definition_counts = new Dictionary<string, int>
            {
                ["card"] = definitions.Count,
                ["relic"] = 0
            },
            definitions,
            adapter_compatibility = "content-index-v1"
        });
        string manifestId = ControlledSnapshot.ManifestId == "reloaded-manifest"
            ? "reloaded-manifest"
            : "2e1dbb4bc0ed23a99d0875d1567575bb6fc2978fc0ea0dfdfce7953a57677230";
        capture = new NativeContentIndexCapture(
            ControlledSnapshot with { ManifestId = manifestId }, sourceJson);
        return true;
    }
}
public static partial class ModEntry
{
    private static nint _nativeLibrary = 0;
    internal static void SetNativeLibraryForTest(nint handle) => _nativeLibrary = handle;
    private const int RuntimeAccepted = 200; private const int RuntimeRejected = 409;
    private static LiveCardCapturedSnapshot _testSnapshot = LiveCardCapturedSnapshot.Unavailable("unset");
    private readonly struct RuntimeContext {
        internal RuntimeContext(string i,string c,string s,string l,string e,string r,string o) { InstanceId=i;CallerId=c;SessionId=s;LeaseId=l;LeaseEpoch=e;CorrelationId=r;Locale=o; }
        internal string InstanceId{get;} internal string CallerId{get;} internal string SessionId{get;} internal string LeaseId{get;} internal string LeaseEpoch{get;} internal string CorrelationId{get;} internal string Locale{get;}
    }
    private static ulong ParseEpoch(string value) => ulong.Parse(value, CultureInfo.InvariantCulture);
    private static string? StringField(System.Text.Json.JsonElement value, string name) =>
        value.TryGetProperty(name, out var result) ? result.GetString() : null;
    private static bool TryAuthorizeRuntimeV2Context(RuntimeContext _, out string error) { error=""; return true; }
    internal static LiveCardCapturedSnapshot ReadRetainedLiveCardSnapshot(string _,string __,ulong ___) => _testSnapshot;
    internal static void SetSnapshot(LiveCardCapturedSnapshot snapshot) => _testSnapshot=snapshot;
    internal static (int,string) Invoke(string body) => ProcessGameInformationQueryWork(new RuntimeContext("instance","caller","session","lease","1","corr","en-US"),body);
    internal static void SeedBinding(LiveCardCapturedSnapshot snapshot) {
        using JsonDocument document=JsonDocument.Parse("{\"project_id\":\"project\",\"run_id\":\"run\",\"episode_id\":\"episode\",\"agent_id\":\"agent\",\"authority_epoch\":1}");
        AssociateLiveCardBinding(new RuntimeContext("instance","caller","session","lease","1","corr","en-US"), document.RootElement, snapshot);
    }
}
