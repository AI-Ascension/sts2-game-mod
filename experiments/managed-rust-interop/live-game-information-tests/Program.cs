// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

if (args.Length > 0)
    Environment.SetEnvironmentVariable("REQUIRE_GAME_INFORMATION_SCHEMA_VALIDATOR", "1");

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
int capabilityStatus = ModEntry.Invoke("").Item1;
string capabilityResponse = ModEntry.Invoke("").Item2;
ProbeHelpers.Check(capabilityStatus == 200, "capabilities response is available");
CanonicalValidation.ValidateCanonicalResponse(capabilityResponse, "capabilities response");

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

string liveWire = ProbeHelpers.Request(
    "detail", "live", "manifest", "en-US", 1, ["display_name"],
    target: liveTarget, parent: liveParent, instance: instanceRef, snapshot: liveSnapshot,
    visibility: "player");
ProbeHelpers.ExpectStatus(liveWire, 200, "real handler returns bound retained detail");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"run_id\":\"run\"", "\"run_id\":\"foreign\"", StringComparison.Ordinal),
    409, "foreign live run is stale");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"epoch\":7", "\"epoch\":8", StringComparison.Ordinal),
    409, "foreign live epoch is stale");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"snapshot_id\":\"snap\"", "\"snapshot_id\":\"foreign\"", StringComparison.Ordinal),
    409, "foreign live snapshot is stale");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"entity_id\":\"card-1\"", "\"entity_id\":\"foreign\"", StringComparison.Ordinal),
    404, "foreign live instance is unknown");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"entity_kind\":\"card\"", "\"entity_kind\":\"relic\"", StringComparison.Ordinal),
    400, "foreign live entity kind is malformed");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"kind\":\"query_request\"", "\"kind\":\"query_request\",\"kind\":\"query_request\"", StringComparison.Ordinal),
    400, "duplicate key rejected");
ProbeHelpers.ExpectStatus(liveWire.Replace("\"kind\":\"query_request\"", "\"kind\":\"query_request\",\"unexpected\":1", StringComparison.Ordinal),
    400, "unknown envelope property rejected");

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
if (args.Length > 0)
{
string staticPageOne = ProbeHelpers.Request("list", "static", requestedStaticManifest, "en-US", 2,
    ["display_name", "rarity", "tags"]);
(int firstStatus, string firstResponse) = ModEntry.Invoke(staticPageOne);
ProbeHelpers.Check(firstStatus == 200, "static list returns first bounded page");
CanonicalValidation.ValidateCanonicalResponse(firstResponse, "static list first page");
using JsonDocument firstDocument = JsonDocument.Parse(firstResponse);
JsonElement firstPage = firstDocument.RootElement.GetProperty("result").GetProperty("page");
ProbeHelpers.Check(firstPage.GetProperty("total_count").GetInt32() == 3
    && !firstPage.GetProperty("final_page").GetBoolean()
    && firstPage.GetProperty("total_count_known").GetBoolean(),
    "static list reports visible total and partial page state");
ProbeHelpers.Check(firstPage.GetProperty("accounting").GetProperty("item_count").GetInt32()
    == firstPage.GetProperty("items").GetArrayLength(),
    "static list item accounting matches emitted items");
CanonicalValidation.ExpectCanonicalReject(
    firstResponse.Replace("\"item_count\":2", "\"item_count\":3", StringComparison.Ordinal),
    "canonical validator rejects inconsistent item accounting");
foreach (string accountingField in new[] { "item_bytes", "payload_bytes", "page_bytes", "text_bytes" })
    CanonicalValidation.ExpectCanonicalReject(
        ProbeHelpers.CorruptAccounting(firstResponse, accountingField),
        $"canonical validator rejects inconsistent {accountingField} accounting");
string staticManifestId = firstPage.GetProperty("items")[0]
    .GetProperty("definition_ref").GetProperty("content_manifest_id").GetString()!;
string nextCursor = firstPage.GetProperty("next_cursor").GetString()!;
string staticPageTwo = staticPageOne.Replace("\"cursor\":null",
    $"\"cursor\":\"{nextCursor}\"", StringComparison.Ordinal);
ProbeHelpers.ExpectStatus(staticPageTwo, 200, "static continuation returns final page");
ProbeHelpers.ExpectStatus(staticPageTwo, 409, "static continuation is single use");
ProbeHelpers.ExpectStatus(staticPageTwo.Replace($"\"{staticManifestId}\"", "\"foreign\"", StringComparison.Ordinal),
    409, "static continuation rejects foreign content binding");
if (args.Length > 0)
{
    string reloadPage = ProbeHelpers.Request("list", "static", requestedStaticManifest, "en-US", 1,
        ["display_name"]);
    (int reloadStatus, string reloadResponse) = ModEntry.Invoke(reloadPage);
    ProbeHelpers.Check(reloadStatus == 200, "native reload probe returns a page");
    using JsonDocument reloadDocument = JsonDocument.Parse(reloadResponse);
    string reloadCursor = reloadDocument.RootElement.GetProperty("result").GetProperty("page")
        .GetProperty("next_cursor").GetString()!;
    string reloadContinuation = reloadPage.Replace("\"cursor\":null",
        $"\"cursor\":\"{reloadCursor}\"", StringComparison.Ordinal);
    NativeContentIndexSnapshot retainedSnapshot =
        NativeContentCatalogManifestSource.ControlledSnapshot!;
    NativeContentCatalogManifestSource.ControlledSnapshot =
        retainedSnapshot with { ManifestId = "reloaded-manifest" };
    ProbeHelpers.ExpectStatus(reloadContinuation, 409, "native continuation rejects source reload");
    NativeContentCatalogManifestSource.ControlledSnapshot = retainedSnapshot;
}

string staticSearch = ProbeHelpers.Request("search", "static", requestedStaticManifest, "en-US", 4,
    ["display_name"], filters: new
    {
        definition_refs = Array.Empty<object>(),
        display_name = "strike",
        instance_ids = Array.Empty<string>(),
        namespaced_ids = Array.Empty<string>()
    });
ProbeHelpers.ExpectStatus(staticSearch, 200, "static search folds display-name text");
string[] selectedNamespacedIds = ["ironclad:strike"];
string staticIdFilter = ProbeHelpers.Request("list", "static", requestedStaticManifest, "en-US", 4,
    ["display_name"], filters: new
    {
        definition_refs = Array.Empty<object>(),
        display_name = (string?)null,
        instance_ids = Array.Empty<string>(),
        namespaced_ids = selectedNamespacedIds
    });
(int idFilterStatus, string idFilterResponse) = ModEntry.Invoke(staticIdFilter);
ProbeHelpers.Check(idFilterStatus == 400, "native static list refuses unsupported namespaced-id filter");
CanonicalValidation.ValidateCanonicalResponse(idFilterResponse, "native identity filter refusal");
using JsonDocument idFilterDocument = JsonDocument.Parse(idFilterResponse);
ProbeHelpers.Check(idFilterDocument.RootElement.GetProperty("error").GetProperty("code")
    .GetString() == "unsupported_filter", "native id filter reports typed refusal");

string staticGet = ProbeHelpers.Request("get", "static", requestedStaticManifest, "en-US", 1,
    ["display_name", "description", "source_id"], target: new
    {
        definition_ref = new
        {
            content_manifest_id = staticManifestId, entity_kind = "card",
            namespaced_id = "ironclad:strike", variant = (string?)null
        },
        instance_ref = (object?)null
    });
ProbeHelpers.ExpectStatus(staticGet, 200, "static get returns exact owned definition");
ProbeHelpers.ExpectStatus(staticGet.Replace("ironclad:strike", "ironclad:missing", StringComparison.Ordinal),
    400, "static get rejects unknown target identity");
ProbeHelpers.ExpectStatus(staticPageOne.Replace("\"page_bytes\":65536", "\"page_bytes\":1", StringComparison.Ordinal),
    413, "static page enforces requested byte bound");
}
