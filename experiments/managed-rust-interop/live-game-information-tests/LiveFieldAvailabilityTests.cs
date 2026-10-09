// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

internal static class LiveFieldAvailabilityTests
{
    internal static void Run()
    {
        LiveCardCapturedCard card = Card();
        ExpectField(card with { ResolvedCost = LiveCardField<int>.Available(0) },
            "cost", "integer", "count", "available", null, JsonValueKind.Number, 0);
        ExpectField(card with { ResolvedCost = LiveCardField<int>.NotObserved() },
            "cost", "integer", "count", "not_observable", "field_not_observed", JsonValueKind.Null);
        ExpectField(card with { ResolvedCost = LiveCardField<int>.Unsupported() },
            "cost", "integer", "count", "unsupported", "field_unsupported", JsonValueKind.Null);
        ExpectField(card with { ResolvedCost = LiveCardField<int>.Unknown() },
            "cost", "integer", "count", "unavailable", "field_value_unknown", JsonValueKind.Null);
        ExpectField(card with { ResolvedCost = LiveCardField<int>.Failed() },
            "cost", "integer", "count", "unavailable", "field_read_failed", JsonValueKind.Null);
        ExpectField(card with { ResolvedCost = new((LiveCardFieldStatus)99, 0) },
            "cost", "integer", "count", "unavailable", "field_status_unrecognized", JsonValueKind.Null);
        ExpectError(card with { ResolvedCost = new(LiveCardFieldStatus.Stale, 0) },
            "cost", 409, "stale_snapshot", "live_field_stale");
        ExpectError(card with { Title = LiveCardField<string>.Available(string.Empty) },
            "display_name", 503, "missing_capability", "live_field_value_invalid");
        ExpectField(card, "tags", "text_list", null, "not_observable",
            "field_not_observed", JsonValueKind.Null);
        ExpectError(card, "cost", 413, "result_limit_exceeded", "requested_page_limit_exceeded", 1);
    }

    private static void ExpectField(
        LiveCardCapturedCard card,
        string name,
        string kind,
        string? unit,
        string availability,
        string? reason,
        JsonValueKind valueKind,
        int? integerValue = null)
    {
        (int status, string response) = Invoke(card, name);
        ProbeHelpers.Check(status == 200, $"live {name} field status is represented");
        CanonicalValidation.ValidateCanonicalResponse(response, $"live {name} field response");
        using JsonDocument document = JsonDocument.Parse(response);
        ProbeHelpers.Check(document.RootElement.GetProperty("query").GetProperty("fields")[0]
            .GetString() == name, $"live {name} response echoes the requested field");
        JsonElement field = document.RootElement.GetProperty("result").GetProperty("page")
            .GetProperty("items")[0].GetProperty("fields")[0];
        ProbeHelpers.Check(field.GetProperty("kind").GetString() == kind
            && field.GetProperty("unit").GetString() == unit, $"live {name} kind and unit are retained");
        string reasonLabel = reason ?? "no_reason";
        ProbeHelpers.Check(field.GetProperty("availability").GetString() == availability
            && field.GetProperty("reason").GetString() == reason,
            $"live {name} status is {availability}/{reasonLabel}");
        JsonElement value = field.GetProperty("value");
        ProbeHelpers.Check(value.ValueKind == valueKind
            && (!integerValue.HasValue || value.GetInt32() == integerValue.Value),
            $"live {name} value is not fabricated");
    }

    private static void ExpectError(
        LiveCardCapturedCard card,
        string name,
        int expectedStatus,
        string code,
        string reason,
        int pageBytes = 65536)
    {
        (int status, string response) = Invoke(card, name, pageBytes);
        ProbeHelpers.Check(status == expectedStatus, $"live {name} refusal status is typed");
        CanonicalValidation.ValidateCanonicalResponse(response, $"live {name} refusal response");
        using JsonDocument document = JsonDocument.Parse(response);
        JsonElement error = document.RootElement.GetProperty("error");
        ProbeHelpers.Check(error.GetProperty("code").GetString() == code
            && error.GetProperty("reason").GetString() == reason,
            $"live {name} refusal preserves {code}/{reason}");
    }

    private static (int Status, string Response) Invoke(
        LiveCardCapturedCard card, string field, int pageBytes = 65536)
    {
        var snapshot = new LiveCardCapturedSnapshot(
            "instance", "manifest", "source", "run", "snap", 7, 9, new[] { card }, true, null);
        ModEntry.SetSnapshot(snapshot);
        ModEntry.SeedBinding(snapshot);
        object instance = new
        {
            instance_id = "instance", run_id = "run", epoch = 7,
            entity_kind = "card", entity_id = card.InstanceId
        };
        object snapshotRef = new { snapshot_id = "snap", state_generation = 9, instance_ref = instance };
        object parent = new { state_generation = 9, snapshot_ref = snapshotRef, instance_ref = instance };
        object target = new
        {
            definition_ref = new
            {
                content_manifest_id = "manifest", entity_kind = "card",
                namespaced_id = "ironclad:strike", variant = (string?)null
            },
            instance_ref = instance
        };
        string request = ProbeHelpers.Request("detail", "live", "manifest", "en-US", 1, [field],
            target: target, parent: parent, instance: instance, snapshot: snapshotRef,
            visibility: "player");
        if (pageBytes != 65536)
            request = request.Replace("\"page_bytes\":65536", $"\"page_bytes\":{pageBytes}",
                StringComparison.Ordinal);
        return ModEntry.Invoke(request);
    }

    private static LiveCardCapturedCard Card() => new(
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
}
