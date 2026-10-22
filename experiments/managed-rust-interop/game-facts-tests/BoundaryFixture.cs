// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class NativeContentCatalogManifestSource
{
    private static readonly string[] EntityKinds = ["card"];
    internal static Action? OnCapture { get; set; }
    internal static ulong Generation { get; set; } = 7;
    internal static string Locale { get; set; } = "en";
    internal static string Semantic { get; set; } = "owned-card-fixture";
    internal static string CaptureJson()
    {
        OnCapture?.Invoke();
        return JsonSerializer.Serialize(new
        {
            generation_before = Generation, generation_after = Generation,
            game_build = "source-probe-build", locale = Locale,
            packages = new[] { new { package_id = "base", package_version = "1", order = 0 } },
            available_entity_kinds = EntityKinds,
            registry_definition_counts = new { card = 1 },
            definitions = new[] { new {
                entity_kind = "card", namespaced_id = "card.fixture", semantic_inputs = Semantic,
                localized_text = "Fixture card", origin = new { package_id = "base", package_version = "1" },
                override_chain = Array.Empty<string>()
            } },
            adapter_compatibility = "sts2-game-mod-modeldb-card-v1"
        });
    }
}

public static partial class ModEntry
{
    private const int RuntimeAccepted = 200;
    private const int RuntimeRejected = 409;
    private const int RuntimeUnavailable = 503;
    private static nint _nativeLibrary;
    private static LiveCardSourceReadAdapter? _reader;
    internal static bool Authorized { get; set; } = true;
    private sealed record RuntimeContext(string InstanceId, string CallerId, string SessionId,
        string LeaseId, string LeaseEpoch, string CorrelationId, string Locale);
    private static readonly RuntimeContext Context = new("instance", "caller", "session", "lease", "1", "facts-probe", "en");

    internal static void Configure(nint library, LiveCardSourceReadAdapter reader)
    {
        _nativeLibrary = library; _reader = reader; Authorized = true; _liveCardBinding = null;
    }

    internal static void Associate(LiveCardCapturedSnapshot snapshot)
    {
        using JsonDocument scope = JsonDocument.Parse("""
            {"project_id":"project","run_id":"harness-run","episode_id":"episode","agent_id":"agent","authority_epoch":1}
            """);
        LookupBindingManifest manifest = LookupBindingManifest.Read(Context.CorrelationId);
        AssociateLiveCardBinding(Context, scope.RootElement, snapshot,
            manifest.CatalogGeneration, manifest.Locale);
    }

    internal static (int Status, string Response) Query(string body) => ProcessGameFactsWork(Context, body);
    internal static string CurrentManifestId()
    {
        return LookupBindingManifest.Read(Context.CorrelationId).ContentManifestId;
    }
    internal static (int Status, string Response) QueryFromWrongLease(string body) =>
        ProcessGameFactsWork(Context with { LeaseId = "other-lease" }, body);
    private static bool TryAuthorizeRuntimeV2Context(RuntimeContext _, out string error)
    { error = string.Empty; return Authorized; }
    private static LiveCardCapturedSnapshot ReadRetainedLiveCardSnapshot(string instance, string manifest, ulong epoch) =>
        _reader?.ReadRetained(instance, manifest, epoch) ?? LiveCardCapturedSnapshot.Unavailable("unconfigured");
    private static ulong ParseEpoch(string value) => ulong.TryParse(value, out ulong epoch) ? epoch : 0;
    private static string? StringField(JsonElement value, string name) =>
        value.TryGetProperty(name, out JsonElement field) ? field.GetString() : null;
}

internal sealed class BoundaryFixture
{
    internal LiveCombatSource Source { get; } = new();
    internal ControlledThread Thread { get; } = new();
    internal LiveCardSourceReadAdapter Reader { get; }
    internal Player Player { get; } = new();
    internal CardModel Card { get; } = new();

    internal BoundaryFixture()
    {
        RunManager.Instance.IsInProgress = true;
        RunManager.Instance.State = new();
        MegaCrit.Sts2.Core.Saves.SaveManager.Instance.IsProfileInitialized = true;
        MegaCrit.Sts2.Core.Saves.SaveManager.Instance.CurrentProfileId = 1;
        MegaCrit.Sts2.Core.Saves.SaveManager.Instance.Progress = new();
        Player.PlayerCombatState!.Hand.Cards.Add(Card);
        LiveCombatSource.Player = Player;
        Reader = new(Source, Thread);
        NativeContentCatalogManifestSource.OnCapture = null;
        NativeContentCatalogManifestSource.Generation = 7;
        NativeContentCatalogManifestSource.Locale = "en";
        NativeContentCatalogManifestSource.Semantic = "owned-card-fixture";
    }
}
