// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // Runs only in the existing authenticated host-thread work dispatcher. No host object
    // enters the native bridge: catalog, occurrence identity and raw values are owned DTOs.
    private static (int Status, string Response) ProcessLiveFactsWork(
        RuntimeContext context, string body, bool capabilities)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out _))
            return FactsRefusal(409, "invalid_binding");
        if (!ContentManifestWireContract.ValidIdentity(context.CorrelationId)
            || !ContentManifestWireContract.ValidLocale(context.Locale))
            return FactsRefusal(400, "invalid_context");
        if (capabilities)
        {
            if (body.Length != 0) return FactsRefusal(400, "invalid_request");
            return ProduceFacts(new { operation = "capabilities", correlation_id = context.CorrelationId,
                request = (object?)null, source = (object?)null, observation = (object?)null });
        }
        if (Encoding.UTF8.GetByteCount(body) > LiveFactsMaxOutputBytes)
            return FactsRefusal(413, "request_limit_exceeded");
        try
        {
            using JsonDocument document = JsonDocument.Parse(body,
                new JsonDocumentOptions { MaxDepth = 32 });
            JsonElement root = document.RootElement;
            if (!UniqueFactsFields(root)
                || root.GetProperty("correlation_id").GetString() != context.CorrelationId)
                return FactsRefusal(400, "invalid_request");
            JsonElement query = root.GetProperty("query");
            JsonElement binding = query.GetProperty("binding");
            if (binding.GetProperty("mode").GetString() != "live")
                return FactsUnavailable(context, root, false);
            JsonElement instance = binding.GetProperty("instance_ref");
            string manifest = binding.GetProperty("content_manifest_id").GetString()!;
            LiveCardCapturedSnapshot snapshot = ReadRetainedLiveCardSnapshot(context.InstanceId,
                manifest, instance.GetProperty("epoch").GetUInt64());
            LiveCardBindingAssociation? association = _liveCardBinding;
            if (!snapshot.Available || association is null
                || !TryReadLiveCardBinding(context, association.RunId, manifest,
                    association.AuthorityEpoch, snapshot)
                || association.Locale != context.Locale || snapshot.Locale != context.Locale
                || snapshot.ManifestWitness is null)
                return FactsUnavailable(context, root, true);
            LiveCardCapturedCard? card = snapshot.Cards.FirstOrDefault(value =>
                value.InstanceId == instance.GetProperty("entity_id").GetString());
            if (card is null) return FactsUnavailable(context, root, true);
            if (!NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndexWithSource(
                    context.CorrelationId, context.Locale, out NativeContentIndexCapture catalog)
                || catalog.Snapshot.ManifestId != snapshot.ContentManifest
                || catalog.Snapshot.Locale != snapshot.Locale
                || catalog.Snapshot.ManifestWitness != snapshot.ManifestWitness)
                return FactsUnavailable(context, root, true);
            // Recheck after catalog extraction as well, without creating a replacement snapshot.
            LiveCardCapturedSnapshot checkedSnapshot = ReadRetainedLiveCardSnapshot(
                context.InstanceId, manifest, snapshot.Epoch);
            if (!ReferenceEquals(snapshot, checkedSnapshot))
                return FactsUnavailable(context, root, true);
            using JsonDocument source = JsonDocument.Parse(catalog.SourceJson);
            // Bootstrap exposes the lookup owner's logical run ID, whose association separately
            // binds the native run incarnation. Never substitute the private native run ID here.
            var ownedInstance = new { instance_id = snapshot.InstanceId, run_id = association.RunId,
                epoch = snapshot.Epoch, entity_kind = "card", entity_id = card.InstanceId };
            var observation = new
            {
                mode_id = snapshot.ModeId,
                manifest = new {
                    catalog_generation = snapshot.ManifestWitness.CatalogGeneration,
                    adapter_compatibility = snapshot.ManifestWitness.AdapterCompatibility,
                    content_set_revision = snapshot.ManifestWitness.ContentSetRevision,
                    localized_text_revision = snapshot.ManifestWitness.LocalizedTextRevision,
                    inventory_revision = snapshot.ManifestWitness.InventoryRevision },
                scope = new { instance_id = association.InstanceId, run_id = association.RunId,
                    authority_epoch = association.AuthorityEpoch,
                    content_manifest_id = snapshot.ContentManifest, locale = snapshot.Locale },
                instance_ref = ownedInstance,
                snapshot_ref = new { snapshot_id = snapshot.SnapshotId, instance_ref = ownedInstance,
                    state_generation = snapshot.StateGeneration },
                energy = FactsValue(snapshot.Energy), card_cost = FactsValue(card.ResolvedCost)
            };
            return ProduceFacts(new { operation = "query", correlation_id = context.CorrelationId,
                request = root, source = source.RootElement, observation });
        }
        catch (JsonException) { return FactsRefusal(400, "invalid_request"); }
        catch (Exception) { return FactsRefusal(503, "source_unavailable"); }
    }

    private static object FactsValue(LiveCardField<int> field) => new
    {
        availability = field.Status switch
        {
            LiveCardFieldStatus.Available => "available",
            LiveCardFieldStatus.Unsupported => "unsupported",
            LiveCardFieldStatus.NotObserved => "not_observable",
            _ => "unavailable"
        },
        value = field.Status == LiveCardFieldStatus.Available ? (long?)field.Value : null
    };

    private static (int Status, string Response) ProduceFacts(object input) =>
        TryProduceLiveFacts(JsonSerializer.Serialize(input), out int status, out string response)
            ? (status, response) : FactsRefusal(503, "native_facts_unavailable");

    private static (int Status, string Response) FactsUnavailable(
        RuntimeContext context, JsonElement request, bool stale) => ProduceFacts(new
        {
            operation = stale ? "reject_stale" : "query", correlation_id = context.CorrelationId,
            request, source = (object?)null, observation = (object?)null
        });

    private static (int Status, string Response) FactsRefusal(int status, string code) =>
        (status, JsonSerializer.Serialize(new { error_code = code }));

    private static bool UniqueFactsFields(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            var names = new HashSet<string>(StringComparer.Ordinal);
            foreach (JsonProperty property in element.EnumerateObject())
                if (!names.Add(property.Name) || !UniqueFactsFields(property.Value)) return false;
        }
        else if (element.ValueKind == JsonValueKind.Array)
            foreach (JsonElement child in element.EnumerateArray())
                if (!UniqueFactsFields(child)) return false;
        return true;
    }
}
