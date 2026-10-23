// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const int RuntimeUnavailable = 503;
    private const int RuntimeAccepted = 200;
    private static nint _nativeLibrary;
    private static bool _authorized = true;
    private static readonly string[] RuleIds = ["resource.fixed_card_cost"];
    private static LiveCardCapturedSnapshot _snapshot = null!;
    private static readonly RuntimeContext Context =
        new("instance", "caller", "session", "lease", "1", "correlation", "en-US");

    private static bool TryAuthorizeRuntimeV2Context(RuntimeContext _, out string error)
    { error = string.Empty; return _authorized; }
    private static ulong ParseEpoch(string value) => ulong.Parse(value, System.Globalization.CultureInfo.InvariantCulture);
    private static string? StringField(JsonElement value, string name) => value.GetProperty(name).GetString();
    internal static LiveCardCapturedSnapshot ReadRetainedLiveCardSnapshot(
        string instance, string manifest, ulong epoch) =>
        _snapshot.Available && _snapshot.InstanceId == instance
        && _snapshot.ContentManifest == manifest && _snapshot.Epoch == epoch
            ? _snapshot : LiveCardCapturedSnapshot.Unavailable("stale_snapshot");

    private static void Check(bool result, string message)
    { if (!result) throw new InvalidOperationException(message); }

    public static int Main(string[] args)
    {
        if (args.Length != 1) throw new ArgumentException("Pass the built native companion path.");
        _nativeLibrary = NativeLibrary.Load(Path.GetFullPath(args[0]));
        try { Run(); Console.WriteLine("Live facts production bridge and association probe passed."); }
        finally { NativeLibrary.Free(_nativeLibrary); _nativeLibrary = 0; }
        return 0;
    }

    private static void Run()
    {
        Reset();
        var caps = ProcessLiveFactsWork(Context, string.Empty, true);
        Check(caps.Status == 200 && caps.Response.Contains("capabilities_response"), "capabilities");
        Check(ProcessLiveFactsWork(Context, "{}", true).Status == 400, "bodyless capabilities");
        _authorized = false;
        Check(ProcessLiveFactsWork(Context, "", true).Status == 409, "capabilities authentication");
        Check(ProcessLiveFactsWork(Context, Query().ToJsonString(), false).Status == 409, "query authentication");
        _authorized = true;

        var found = Call(Query());
        Check(found.Status == 200, "fixed costs found: " + found.Response);
        using (JsonDocument response = JsonDocument.Parse(found.Response))
        {
            JsonElement result = response.RootElement.GetProperty("result").GetProperty("results")[0];
            Check(result.GetProperty("evidence_status").GetString() == "source_derived", "not parity");
            string inputs = result.GetProperty("inputs").GetRawText();
            Check(inputs.Contains("70000") && inputs.Contains("90000"), "raw unclamped values");
        }
        JsonObject extra = Query(); extra["unexpected"] = 1;
        Check(Call(extra).Status != 200, "closed codec envelope");
        string duplicate = Query().ToJsonString().Replace("\"kind\":\"query_request\"",
            "\"kind\":\"query_request\",\"kind\":\"query_request\"");
        Check(ProcessLiveFactsWork(Context, duplicate, false).Status == 400, "duplicate fields");
        foreach (string field in new[] { "instance_id", "run_id", "content_manifest_id", "locale" })
        {
            JsonObject changed = Query(); changed["query"]!["scope"]![field] = "foreign";
            Check(Call(changed).Status != 200, "scope fence " + field);
        }
        foreach (RuntimeContext changed in new[] {
            new RuntimeContext("instance", "other", "session", "lease", "1", "correlation", "en-US"),
            new RuntimeContext("instance", "caller", "other", "lease", "1", "correlation", "en-US"),
            new RuntimeContext("instance", "caller", "session", "other", "1", "correlation", "en-US"),
            new RuntimeContext("instance", "caller", "session", "lease", "2", "correlation", "en-US") })
            Check(ProcessLiveFactsWork(changed, Query().ToJsonString(), false).Status == 409,
                "retained association transport fence");
        JsonObject stale = Query();
        stale["query"]!["binding"]!["instance_ref"]!["epoch"] = 2;
        stale["query"]!["binding"]!["snapshot_ref"]!["instance_ref"]!["epoch"] = 2;
        stale["query"]!["parent_observation"]!["instance_ref"]!["epoch"] = 2;
        stale["query"]!["parent_observation"]!["snapshot_ref"]!["instance_ref"]!["epoch"] = 2;
        Check(Call(stale).Status == 409, "selected epoch fence");
        JsonObject staleId = Query();
        staleId["query"]!["binding"]!["snapshot_ref"]!["snapshot_id"] = "retired";
        staleId["query"]!["parent_observation"]!["snapshot_ref"]!["snapshot_id"] = "retired";
        Check(Call(staleId).Status == 409, "selected snapshot-id fence");
        JsonObject staleGeneration = Query();
        staleGeneration["query"]!["binding"]!["snapshot_ref"]!["state_generation"] = 5;
        staleGeneration["query"]!["parent_observation"]!["snapshot_ref"]!["state_generation"] = 5;
        staleGeneration["query"]!["parent_observation"]!["state_generation"] = 5;
        Check(Call(staleGeneration).Status == 409, "selected state-generation fence");
        Check(LiveFactsMaxInputBytes > 16 * 1024 * 1024 + LiveFactsMaxOutputBytes,
            "native input must fit full source plus request envelope");
        _snapshot = _snapshot with { Energy = LiveCardField<int>.NotObserved() };
        var unavailable = Call(Query());
        Check(unavailable.Status == 200 && !unavailable.Response.Contains("70000"), "missing energy not zero");
        _snapshot = _snapshot with { Cards = new[] { _snapshot.Cards[0] with {
            ResolvedCost = LiveCardField<int>.Unsupported() } } };
        var x = Call(Query());
        Check(x.Status == 200 && !x.Response.Contains("90000"), "unsupported cost not fabricated");
        Reset(); NativeContentCatalogManifestSource.Revision++;
        Check(Call(Query()).Status == 409, "canonical current content identity");
        Reset(); NativeContentCatalogManifestSource.Locale = "fr-FR";
        Check(Call(Query()).Status == 409, "current locale fence");
        Reset(); NativeContentCatalogManifestSource.OnCapture = () =>
            _snapshot = LiveCardCapturedSnapshot.Unavailable("changed_during_catalog");
        Check(Call(Query()).Status == 409, "post-catalog retained recheck");
        Reset(); _liveCardBinding = null;
        Check(Call(Query()).Status == 409, "association required");
    }

    private static (int Status, string Response) Call(JsonObject query) =>
        ProcessLiveFactsWork(Context, query.ToJsonString(), false);

    private static void Reset()
    {
        NativeContentCatalogManifestSource.Locale = "en-US";
        NativeContentCatalogManifestSource.Revision = 0;
        NativeContentCatalogManifestSource.OnCapture = null;
        Check(NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndexWithSource(
            "correlation", "en-US", out NativeContentIndexCapture capture), "real manifest producer");
        var card = new LiveCardCapturedCard("card1", LiveCardField<string>.Available("strike"),
            LiveCardField<string>.Available(capture.Snapshot.ManifestId),
            LiveCardField<string>.Available("player"),
            LiveCardField<LiveCardLocation>.Available(new(LiveCardZone.Hand, 0)),
            LiveCardField<ushort>.Available(0), LiveCardField<string>.NotObserved(),
            LiveCardField<string>.NotObserved(), LiveCardField<string>.Available("Strike"),
            LiveCardField<bool>.Available(false), LiveCardField<int>.Available(90000),
            LiveCardField<int>.NotObserved(), LiveCardField<int>.NotObserved(),
            LiveCardField<IReadOnlyList<string>>.NotObserved(),
            LiveCardField<IReadOnlyList<string>>.NotObserved(),
            LiveCardField<IReadOnlyDictionary<string, string>>.NotObserved());
        _snapshot = new("instance", capture.Snapshot.ManifestId, "source", "native-run", "snapshot", 1, 4,
            new[] { card }, true, null)
        { Energy = LiveCardField<int>.Available(70000), ModeId = "standard", Locale = "en-US",
            ManifestWitness = capture.Snapshot.ManifestWitness };
        AssociateLiveCardBinding(Context, JsonSerializer.SerializeToElement(new {
            project_id = "project", run_id = "run", episode_id = "episode", agent_id = "agent",
            authority_epoch = 1 }), _snapshot);
    }

    private static JsonObject Query()
    {
        var instance = new { instance_id = "instance", run_id = "run", epoch = 1,
            entity_kind = "card", entity_id = "card1" };
        var snapshot = new { snapshot_id = "snapshot", instance_ref = instance, state_generation = 4 };
        return JsonSerializer.SerializeToNode(new {
            protocol_version = "game-facts-reference-v1",
            schema_digest = "3065a2ff96e6e5af628b3d5cb43b8f2db4232ae4c4dc71324b973419908a2985",
            provenance = new { artifact = "sts2-protocol/game-facts-reference-v1",
                source = "schemas/game-facts-reference-v1.schema.json", generator = "hand-authored" },
            correlation_id = "correlation", kind = "query_request",
            query = new { rules_reference_version = 2,
                rule_ids = RuleIds,
                binding = new { mode = "live", content_manifest_id = _snapshot.ContentManifest,
                    locale = "en-US", visibility_scope = "player_visible", instance_ref = instance,
                    snapshot_ref = snapshot },
                scope = new { instance_id = "instance", run_id = "run", authority_epoch = 1,
                    content_manifest_id = _snapshot.ContentManifest, locale = "en-US" },
                parent_observation = new { instance_ref = instance, snapshot_ref = snapshot,
                    state_generation = 4 } },
            result = (object?)null, capabilities = (object?)null, error = (object?)null
        })!.AsObject();
    }
}
