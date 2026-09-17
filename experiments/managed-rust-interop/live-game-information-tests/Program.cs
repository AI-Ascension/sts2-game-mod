using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

const string Digest =
    "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";

static string Request(
    string kind,
    string mode,
    string manifest,
    string locale,
    int pageItems,
    string[] fields,
    string? cursor = null,
    object? target = null,
    object? parent = null,
    object? instance = null,
    object? snapshot = null,
    string visibility = "public",
    object? filters = null)
{
    return JsonSerializer.Serialize(new
    {
        protocol_version = "game-information-query-v1",
        schema_digest = Digest,
        provenance = new
        {
            artifact = "sts2-protocol/game-information-query-v1",
            source = "schemas/game-information-query-v1.schema.json",
            generator = "hand-authored"
        },
        correlation_id = "corr",
        kind = "query_request",
        capabilities = (object?)null,
        error = (object?)null,
        result = (object?)null,
        query = new
        {
            cursor,
            detail_level = "summary",
            projection = "summary",
            query_kind = kind,
            entity_kind = "card",
            fields,
            filters = filters ?? new
            {
                definition_refs = Array.Empty<object>(),
                display_name = (string?)null,
                instance_ids = Array.Empty<string>(),
                namespaced_ids = Array.Empty<string>()
            },
            limits = new { item_bytes = 4096, page_bytes = 65536, page_items = pageItems, text_bytes = 4096 },
            binding = new
            {
                mode,
                content_manifest_id = manifest,
                locale,
                visibility_scope = visibility,
                instance_ref = instance,
                snapshot_ref = snapshot
            },
            target = target ?? new { definition_ref = (object?)null, instance_ref = (object?)null },
            parent_observation = parent
        }
    });
}

static void Check(bool condition, string message)
{
    if (!condition)
        throw new InvalidOperationException(message);
    Console.WriteLine($"PASS: {message}");
}

static void ExpectStatus(string body, int expected, string message)
{
    (int status, string response) = ModEntry.Invoke(body);
    Check(status == expected, $"{message} ({status})");
    if (expected == 200)
        Check(response.Contains("\"kind\":\"query_response\"", StringComparison.Ordinal),
            $"{message} has query response envelope");
}

var card = new LiveCardCapturedCard(
    "card-1",
    LiveCardField<string>.Available("ironclad:strike"),
    LiveCardField<string>.Available("manifest"),
    LiveCardField<string>.Available("owner"),
    LiveCardField<LiveCardLocation>.Available(new(LiveCardZone.Hand, 0)),
    LiveCardField<ushort>.Available(0),
    LiveCardField<string>.NotObserved(),
    LiveCardField<string>.NotObserved(),
    LiveCardField<string>.Available("Strike"),
    LiveCardField<bool>.Available(false),
    LiveCardField<int>.Available(1),
    LiveCardField<int>.NotObserved(),
    LiveCardField<int>.NotObserved(),
    LiveCardField<IReadOnlyList<string>>.NotObserved(),
    LiveCardField<IReadOnlyList<string>>.NotObserved(),
    LiveCardField<IReadOnlyDictionary<string, string>>.NotObserved());
var snapshot = new LiveCardCapturedSnapshot(
    "instance", "manifest", "source", "run", "snap", 7, 9, new[] { card }, true, null);
ModEntry.SetSnapshot(snapshot);
ModEntry.SeedBinding(snapshot);

object instanceRef = new
{
    instance_id = "instance", run_id = "run", epoch = 7,
    entity_kind = "card", entity_id = "card-1"
};
object liveTarget = new
{
    definition_ref = new
    {
        content_manifest_id = "manifest", entity_kind = "card",
        namespaced_id = "ironclad:strike", variant = (string?)null
    },
    instance_ref = instanceRef
};
object liveSnapshot = new
{
    snapshot_id = "snap", state_generation = 9, instance_ref = instanceRef
};
object liveParent = new
{
    state_generation = 9, snapshot_ref = liveSnapshot, instance_ref = instanceRef
};

string liveWire = Request(
    "detail", "live", "manifest", "en-US", 1, ["display_name"],
    target: liveTarget, parent: liveParent, instance: instanceRef, snapshot: liveSnapshot,
    visibility: "player");
ExpectStatus(liveWire, 200, "real handler returns bound retained detail");
ExpectStatus(liveWire.Replace("\"run_id\":\"run\"", "\"run_id\":\"foreign\"", StringComparison.Ordinal),
    409, "foreign live run is stale");
ExpectStatus(liveWire.Replace("\"epoch\":7", "\"epoch\":8", StringComparison.Ordinal),
    409, "foreign live epoch is stale");
ExpectStatus(liveWire.Replace("\"snapshot_id\":\"snap\"", "\"snapshot_id\":\"foreign\"", StringComparison.Ordinal),
    409, "foreign live snapshot is stale");
ExpectStatus(liveWire.Replace("\"entity_id\":\"card-1\"", "\"entity_id\":\"foreign\"", StringComparison.Ordinal),
    404, "foreign live instance is unknown");
ExpectStatus(liveWire.Replace("\"entity_kind\":\"card\"", "\"entity_kind\":\"relic\"", StringComparison.Ordinal),
    400, "foreign live entity kind is malformed");
ExpectStatus(liveWire.Replace("\"kind\":\"query_request\"", "\"kind\":\"query_request\",\"kind\":\"query_request\"", StringComparison.Ordinal),
    400, "duplicate key rejected");

NativeContentCatalogManifestSource.ControlledSnapshot = new NativeContentIndexSnapshot(
    "manifest", "en-US",
    new[]
    {
        new NativeContentIndexDefinition("card", "ironclad:bash", "Bash",
            Array.Empty<string>(), "Deal damage", null, "attack", "unlocked",
            Array.Empty<string>(), null),
        new NativeContentIndexDefinition("card", "ironclad:defend", "Defend",
            new[] { "Guard" }, null, null, null, "unlocked",
            Array.Empty<string>(), null),
        new NativeContentIndexDefinition("card", "ironclad:strike", "Strike",
            new[] { "Hit" }, "Deal damage", null, "attack", "unlocked",
            Array.Empty<string>(), null),
        new NativeContentIndexDefinition("card", "ironclad:hidden", "Hidden",
            Array.Empty<string>(), null, null, null, "unknown",
            Array.Empty<string>(), null)
    });
if (args.Length > 0)
    ModEntry.SetNativeLibraryForTest(NativeLibrary.Load(args[0]));

string requestedStaticManifest = args.Length > 0
    ? "2e1dbb4bc0ed23a99d0875d1567575bb6fc2978fc0ea0dfdfce7953a57677230"
    : "manifest";
string staticPageOne = Request("list", "static", requestedStaticManifest, "en-US", 2,
    ["display_name", "rarity", "tags"]);
(int firstStatus, string firstResponse) = ModEntry.Invoke(staticPageOne);
Check(firstStatus == 200, "static list returns first bounded page");
using JsonDocument firstDocument = JsonDocument.Parse(firstResponse);
JsonElement firstPage = firstDocument.RootElement.GetProperty("result").GetProperty("page");
Check(firstPage.GetProperty("total_count").GetInt32() == 3
    && !firstPage.GetProperty("final_page").GetBoolean()
    && firstPage.GetProperty("total_count_known").GetBoolean(),
    "static list reports visible total and partial page state");
Check(firstPage.GetProperty("accounting").GetProperty("item_count").GetInt32()
    == firstPage.GetProperty("items").GetArrayLength(),
    "static list item accounting matches emitted items");
string staticManifestId = firstPage.GetProperty("items")[0]
    .GetProperty("definition_ref").GetProperty("content_manifest_id").GetString()!;
string nextCursor = firstPage.GetProperty("next_cursor").GetString()!;
string staticPageTwo = staticPageOne.Replace("\"cursor\":null",
    $"\"cursor\":\"{nextCursor}\"", StringComparison.Ordinal);
ExpectStatus(staticPageTwo, 200, "static continuation returns final page");
ExpectStatus(staticPageTwo, 409, "static continuation is single use");
ExpectStatus(staticPageTwo.Replace($"\"{staticManifestId}\"", "\"foreign\"", StringComparison.Ordinal),
    409, "static continuation rejects foreign content binding");
if (args.Length > 0)
{
    string reloadPage = Request("list", "static", requestedStaticManifest, "en-US", 1,
        ["display_name"]);
    (int reloadStatus, string reloadResponse) = ModEntry.Invoke(reloadPage);
    Check(reloadStatus == 200, "native reload probe returns a page");
    using JsonDocument reloadDocument = JsonDocument.Parse(reloadResponse);
    string reloadCursor = reloadDocument.RootElement.GetProperty("result").GetProperty("page")
        .GetProperty("next_cursor").GetString()!;
    string reloadContinuation = reloadPage.Replace("\"cursor\":null",
        $"\"cursor\":\"{reloadCursor}\"", StringComparison.Ordinal);
    NativeContentIndexSnapshot retainedSnapshot =
        NativeContentCatalogManifestSource.ControlledSnapshot!;
    NativeContentCatalogManifestSource.ControlledSnapshot =
        retainedSnapshot with { ManifestId = "reloaded-manifest" };
    ExpectStatus(reloadContinuation, 409, "native continuation rejects source reload");
    NativeContentCatalogManifestSource.ControlledSnapshot = retainedSnapshot;
}

string staticSearch = Request("search", "static", requestedStaticManifest, "en-US", 4,
    ["display_name"], filters: new
    {
        definition_refs = Array.Empty<object>(),
        display_name = "strike",
        instance_ids = Array.Empty<string>(),
        namespaced_ids = Array.Empty<string>()
    });
ExpectStatus(staticSearch, 200, "static search folds display-name text");

string staticGet = Request("get", "static", requestedStaticManifest, "en-US", 1,
    ["display_name", "description", "source_id"], target: new
    {
        definition_ref = new
        {
            content_manifest_id = staticManifestId, entity_kind = "card",
            namespaced_id = "ironclad:strike", variant = (string?)null
        },
        instance_ref = (object?)null
    });
ExpectStatus(staticGet, 200, "static get returns exact owned definition");
ExpectStatus(staticGet.Replace("ironclad:strike", "ironclad:missing", StringComparison.Ordinal),
    400, "static get rejects unknown target identity");
ExpectStatus(staticPageOne.Replace("\"page_bytes\":65536", "\"page_bytes\":1", StringComparison.Ordinal),
    413, "static page enforces requested byte bound");
