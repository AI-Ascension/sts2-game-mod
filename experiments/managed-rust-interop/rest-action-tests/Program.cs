// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static class Program
{
    private const string ProposalRelativePath = "docs/proposals/runtime-v4-expert-rest-action-v1";
    private static readonly string[] FirstSelected = { "card:1" };
    private static readonly string[] FirstProgressActions =
        { "select_card:11:smith:card:2", "cancel_selection:11:smith" };
    private static readonly string[] SecondSelected = { "card:1", "card:2" };
    private static readonly string[] SecondProgressActions =
        { "confirm_selection:12:smith", "cancel_selection:12:smith" };

    private static void Main()
    {
        string proposal = FindProposalDirectory();
        string schemaPath = Path.Combine(proposal, "schema.json");
        string digest = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(schemaPath)))
            .ToLowerInvariant();
        Check(digest == RuntimeV4ExpertRestActionContract.SchemaDigest,
            "managed digest does not match the exact proposal schema bytes");

        string golden = Path.Combine(proposal, "golden");
        var fixtures = new Dictionary<string, JsonDocument>(StringComparer.Ordinal);
        try
        {
            foreach (string path in Directory.EnumerateFiles(golden, "*.json").OrderBy(path => path,
                         StringComparer.Ordinal))
            {
                string name = Path.GetFileName(path);
                JsonDocument document = JsonDocument.Parse(File.ReadAllText(path),
                    new JsonDocumentOptions { MaxDepth = 24 });
                fixtures.Add(name, document);
                ValidateFixture(name, document.RootElement);
            }

            SmithSequenceChecks(fixtures);
            EarlyConfirmationCheck(fixtures);
            SerializedResponseChecks();
            ProducerConsumerSupportChecks();
        }
        finally
        {
            foreach (JsonDocument document in fixtures.Values) document.Dispose();
        }

        Console.WriteLine("Runtime-v4 expert rest action goldens and Smith sequence checks passed.");
    }

    private static void EarlyConfirmationCheck(Dictionary<string, JsonDocument> fixtures)
    {
        JsonElement response = fixtures["action-selection-early-confirm-rejected.json"].RootElement;
        Check(response.GetProperty("status").GetString() == "rejected",
            "early Smith confirmation was not rejected");
        Check(StringField(response.GetProperty("action").GetProperty("action"), "kind")
                == "confirm_selection",
            "early Smith rejection did not echo the control action");
        Check(StringField(response, "error_code") == "sts2.game-mod/rest_selection_incomplete",
            "early Smith confirmation has the wrong rejection code");
        JsonElement initial = fixtures["action-selection-requested.json"].RootElement
            .GetProperty("transition").GetProperty("selector");
        Check(!initial.GetProperty("legal_actions").EnumerateArray()
                .Any(action => StringField(action.GetProperty("action"), "kind")
                    == "confirm_selection"),
            "initial Smith selector still advertises early confirmation");
    }

    private static void ValidateFixture(string name, JsonElement root)
    {
        Check(root.ValueKind == JsonValueKind.Object, name + " must have an object root");
        string? kind = StringField(root, "kind");
        Check(kind is "action_request" or "action_response", name + " has an unsupported envelope kind");
        RuntimeV4ExpertRestContext context = Context(root);
        string json = root.GetRawText();
        if (kind == "action_request")
        {
            Check(RuntimeV4ExpertRestActionCodec.TryParseRequest(json, context,
                    out RuntimeV4ExpertRestRequest? request, out string error),
                name + " request rejected: " + error);
            Check(RuntimeV4ExpertRestActionCodec.TrySerializeRequest(request!, out string serialized,
                    out error), name + " request failed to serialize: " + error);
            Check(Canonical(serialized) == Canonical(json), name + " request round-trip changed its wire shape");
            return;
        }

        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(json, context, out string responseError),
            name + " response rejected: " + responseError);
    }

    private static void SmithSequenceChecks(Dictionary<string, JsonDocument> fixtures)
    {
        Pair(fixtures, "action-selection-option-request.json", "action-selection-requested.json", 9, 10);
        Pair(fixtures, "action-selection-first-request.json", "action-selection-progressed.json", 10, 11);
        Pair(fixtures, "action-selection-second-request.json", "action-selection-second-progressed.json", 11, 12);
        Pair(fixtures, "action-selection-confirm-request.json", "action-selection-completed.json", 12, 13);

        CatalogAction(fixtures, "action-selection-requested.json", "action-selection-first-request.json",
            "select_card:10:smith:card:1", "select_card:10:smith:card:1");
        CatalogAction(fixtures, "action-selection-progressed.json", "action-selection-second-request.json",
            "select_card:11:smith:card:2", "select_card:11:smith:card:2");
        CatalogAction(fixtures, "action-selection-second-progressed.json", "action-selection-confirm-request.json",
            "confirm_selection:12:smith", "confirm_selection:12:smith");

        SelectorMirror(fixtures, "action-selection-progressed.json", 11,
            FirstSelected, 1, FirstProgressActions);
        SelectorMirror(fixtures, "action-selection-second-progressed.json", 12,
            SecondSelected, 0, SecondProgressActions);

        JsonElement final = fixtures["action-selection-completed.json"].RootElement;
        Check(StringField(final.GetProperty("effect_witness"), "operation_id")
                == StringField(final, "operation_id"),
            "final Smith witness is not bound to the confirmation operation");
        Check(final.GetProperty("effect_witness").GetProperty("generation").GetUInt64()
                == final.GetProperty("generation").GetUInt64(),
            "final Smith witness is not bound to the settled generation");
        Check(final.GetProperty("transition").GetProperty("selected_choice_ids").GetArrayLength() == 2,
            "final Smith transition does not retain both card selections");
    }

    private static void SerializedResponseChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:serialized");
        RuntimeV4ExpertRestOperation selectionOperation = new(
            context.InstanceId, context.SessionId, context.LeaseId, context.LeaseEpoch,
            "rest-select:serialized:smith:card:2");
        RuntimeV4ExpertRestActionReference secondPick = new(
            "select_card:11:smith:card:2",
            new RuntimeV4ExpertRestAction("select_card", "smith", "selection:10:smith", "card:2"));
        RuntimeV4ExpertRestSelector progressedSelector = new(
            "selection:10:smith", "card", 2, FirstSelected, 1,
            new[]
            {
                secondPick,
                new RuntimeV4ExpertRestActionReference("cancel_selection:11:smith",
                    new RuntimeV4ExpertRestAction("cancel_selection", "smith", "selection:10:smith"))
            });
        var progressed = new RuntimeV4ExpertRestResponse(
            context, "live:12", 12, selectionOperation, secondPick, "settled", Observation(12),
            new RuntimeV4ExpertRestSelectionProgressedTransition("smith", 11, 12,
                progressedSelector), null, null);
        Check(RuntimeV4ExpertRestActionCodec.TrySerializeResponse(progressed, out string progressedJson,
                out string progressedError), "typed progressed producer failed: " + progressedError);
        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(progressedJson, context,
                out progressedError), "typed progressed consumer rejected: " + progressedError);

        RuntimeV4ExpertRestOperation confirmationOperation = selectionOperation with
        {
            OperationId = "rest-select:serialized:smith:confirm"
        };
        RuntimeV4ExpertRestActionReference confirmation = new(
            "confirm_selection:12:smith",
            new RuntimeV4ExpertRestAction("confirm_selection", "smith", "selection:10:smith"));
        var witness = new RuntimeV4ExpertRestEffectWitness(
            "smith_applied", confirmationOperation, "smith", 13,
            new RuntimeV4ExpertRestCardEvidence(Array.Empty<string>(), Array.Empty<string>(),
                SecondSelected));
        var completed = new RuntimeV4ExpertRestResponse(
            context, "live:13", 13, confirmationOperation, confirmation, "settled", Observation(13),
            new RuntimeV4ExpertRestSelectionCompletedTransition("smith", 12, 13,
                "selection:10:smith", "card", 2, SecondSelected, witness), witness, null);
        Check(RuntimeV4ExpertRestActionCodec.TrySerializeResponse(completed, out string completedJson,
                out string completedError), "typed completion producer failed: " + completedError);
        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(completedJson, context,
                out completedError), "typed completion consumer rejected: " + completedError);
    }

    private static RuntimeV4ExpertGameplayObservation Observation(ulong generation) =>
        new("live:" + generation, generation, "synthetic-visible-seed",
            new RuntimeV4ExpertGameplayRun("ironclad", 1, "rest:1"),
            new RuntimeV4ExpertGameplayPlayer(64, 80, null, 0, 99)
            {
                Hand = Array.Empty<RuntimeV4ExpertGameplayCard>(),
                Deck = Array.Empty<RuntimeV4ExpertGameplayCard>(),
                Discard = Array.Empty<RuntimeV4ExpertGameplayCard>(),
                Exhaust = Array.Empty<RuntimeV4ExpertGameplayCard>(),
                Powers = Array.Empty<RuntimeV4ExpertGameplayStatus>(),
                Statuses = Array.Empty<RuntimeV4ExpertGameplayStatus>(),
                Relics = Array.Empty<RuntimeV4ExpertGameplayRelic>(),
                Potions = Array.Empty<RuntimeV4ExpertGameplayPotion>()
            },
            new RuntimeV4ExpertGameplayState("selection")
            {
                Choices = Array.Empty<RuntimeV4ExpertGameplayChoice>()
            },
            Array.Empty<RuntimeV4ExpertGameplayAction>());

    private static void ProducerConsumerSupportChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:support");
        RuntimeV4ExpertRestOperation operation = new(
            context.InstanceId, context.SessionId, context.LeaseId, context.LeaseEpoch,
            "rest-op:support:heal");
        RuntimeV4ExpertRestActionReference action = new(
            "rest-option:9:heal", new RuntimeV4ExpertRestAction("rest_option", "heal"));
        RuntimeV4ExpertRestRequest request = new(context, operation, action, "live:9", 9);
        Check(RuntimeV4ExpertRestActionCodec.TrySerializeRequest(request, out string body,
                out string error), "support request failed to serialize: " + error);

        var host = new FakeRestHost(request);
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());
        (int Status, string Response) accepted = support.Handle(context, body, out int status);
        Check(status == 200 && accepted.Status == 200, "support request was not accepted");
        using (JsonDocument acceptedDocument = JsonDocument.Parse(accepted.Response))
            Check(acceptedDocument.RootElement.GetProperty("status").GetString() == "accepted",
                "support request did not return accepted receipt");

        (int Status, string Response) replay = support.Handle(context, body, out status);
        Check(status == 200 && replay.Response == accepted.Response,
            "support idempotency replay changed the accepted receipt");

        (int Status, string Response) settled = support.Handle(context, operation.OperationId,
            out status);
        Check(status == 200, "support reconciliation did not settle");
        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(settled.Response, context,
                out error), "support serialized response failed consumer validation: " + error);
        using (JsonDocument settledDocument = JsonDocument.Parse(settled.Response))
        {
            Check(settledDocument.RootElement.GetProperty("status").GetString() == "settled",
                "support reconciliation did not return settled");
            Check(settledDocument.RootElement.GetProperty("transition")
                .GetProperty("kind").GetString() == "rest_option_completed",
                "support reconciliation returned the wrong transition");
        }
        Check(host.DispatchCount == 1 && host.CompletionCount == 1,
            "support dispatched or completed the operation more than once");
    }

    private sealed class FakeRestHost : IRuntimeV4ExpertRestHostSource
    {
        private readonly RuntimeV4ExpertRestRequest _request;
        private bool _dispatched;

        internal FakeRestHost(RuntimeV4ExpertRestRequest request) => _request = request;

        internal int DispatchCount { get; private set; }
        internal int CompletionCount { get; private set; }

        public RuntimeV4ExpertRestHostProjection ObserveRest() =>
            new(Observation(_request.Generation), new[] { _request.Action });

        public bool DispatchRest(RuntimeV4ExpertRestOperation operation,
            RuntimeV4ExpertRestActionReference action, RuntimeV4ExpertRestHostProjection current)
        {
            if (_dispatched || operation != _request.Operation || action != _request.Action)
                return false;
            _dispatched = true;
            DispatchCount++;
            return true;
        }

        public RuntimeV4ExpertRestHostCompletion? CompleteRest(
            RuntimeV4ExpertRestOperation operation, RuntimeV4ExpertRestActionReference action)
        {
            if (!_dispatched || operation != _request.Operation || action != _request.Action)
                return null;
            CompletionCount++;
            RuntimeV4ExpertGameplayObservation before = Observation(9);
            RuntimeV4ExpertGameplayObservation after = Observation(10) with
            {
                Player = before.Player with { Hp = 70 }
            };
            var witness = new RuntimeV4ExpertRestEffectWitness(
                "heal_applied", operation, "heal", 10,
                new RuntimeV4ExpertRestHpEvidence(64, 70, 80, 80));
            var transition = new RuntimeV4ExpertRestCompletedTransition(
                "heal", 9, 10, witness);
            return new RuntimeV4ExpertRestHostCompletion(
                "settled", after, transition, witness, null);
        }
    }

    private static void Pair(
        Dictionary<string, JsonDocument> fixtures,
        string requestName,
        string responseName,
        ulong requestGeneration,
        ulong responseGeneration)
    {
        JsonElement request = fixtures[requestName].RootElement;
        JsonElement response = fixtures[responseName].RootElement;
        Check(request.GetProperty("generation").GetUInt64() == requestGeneration,
            requestName + " has the wrong request generation");
        Check(response.GetProperty("generation").GetUInt64() == responseGeneration,
            responseName + " has the wrong response generation");
        Check(StringField(request, "operation_id") == StringField(response, "operation_id"),
            requestName + " and " + responseName + " use different operation IDs");
        Check(Canonical(request.GetProperty("action")) == Canonical(response.GetProperty("action")),
            requestName + " action is not echoed exactly by " + responseName);
        Check(response.GetProperty("status").GetString() == "settled",
            responseName + " is not a settled producer response");
    }

    private static void CatalogAction(
        Dictionary<string, JsonDocument> fixtures,
        string responseName,
        string requestName,
        string expectedActionId,
        string expectedRequestActionId)
    {
        JsonElement response = fixtures[responseName].RootElement;
        JsonElement request = fixtures[requestName].RootElement;
        JsonElement actions = response.GetProperty("transition").GetProperty("selector")
            .GetProperty("legal_actions");
        JsonElement? found = null;
        foreach (JsonElement candidate in actions.EnumerateArray())
        {
            if (StringField(candidate, "action_id") == expectedActionId)
            {
                found = candidate;
                break;
            }
        }
        Check(found.HasValue, responseName + " does not advertise " + expectedActionId);
        Check(StringField(request.GetProperty("action"), "action_id") == expectedRequestActionId,
            requestName + " uses an unexpected opaque action ID");
        Check(Canonical(found!.Value) == Canonical(request.GetProperty("action")),
            requestName + " does not copy the prior selector catalog entry exactly");
    }

    private static void SelectorMirror(
        Dictionary<string, JsonDocument> fixtures,
        string responseName,
        ulong generation,
        IReadOnlyList<string> selected,
        int remaining,
        IReadOnlyList<string> actionIds)
    {
        JsonElement transition = fixtures[responseName].RootElement.GetProperty("transition");
        JsonElement selector = transition.GetProperty("selector");
        Check(transition.GetProperty("after_generation").GetUInt64() == generation,
            responseName + " transition is not bound to its response generation");
        Check(selector.GetProperty("selection_id").GetString() == "selection:10:smith",
            responseName + " changed the Smith selection identity");
        Check(selector.GetProperty("selection_kind").GetString() == "card",
            responseName + " does not expose a card selector");
        Check(selector.GetProperty("required_count").GetInt32() == 2,
            responseName + " has the wrong Smith required count");
        Check(selector.GetProperty("remaining_count").GetInt32() == remaining,
            responseName + " has the wrong Smith remaining count");
        Check(selector.GetProperty("selected_choice_ids").EnumerateArray()
                .Select(value => value.GetString()!).SequenceEqual(selected),
            responseName + " does not preserve the selected card IDs");
        Check(selector.GetProperty("legal_actions").EnumerateArray()
                .Select(value => value.GetProperty("action_id").GetString()!)
                .SequenceEqual(actionIds), responseName + " does not expose the fresh ordered catalog");
        foreach (string field in new[] { "selection_id", "selection_kind", "required_count", "selected_choice_ids", "remaining_count" })
            Check(Canonical(transition.GetProperty(field)) == Canonical(selector.GetProperty(field)),
                responseName + " has a selector field that disagrees with its refreshed snapshot: " + field);
    }

    private static RuntimeV4ExpertRestContext Context(JsonElement root) => new(
        StringField(root, "instance_id")!, StringField(root, "session_id")!,
        StringField(root, "lease_id")!, root.GetProperty("lease_epoch").GetUInt64(),
        StringField(root, "correlation_id")!);

    private static string FindProposalDirectory()
    {
        foreach (string start in new[] { Directory.GetCurrentDirectory(), AppContext.BaseDirectory })
        {
            DirectoryInfo? directory = new(start);
            for (int depth = 0; directory is not null && depth < 12; depth++, directory = directory.Parent)
            {
                string candidate = Path.Combine(directory.FullName, ProposalRelativePath);
                if (File.Exists(Path.Combine(candidate, "schema.json"))) return candidate;
            }
        }
        throw new InvalidOperationException("proposal directory could not be located");
    }

    private static string? StringField(JsonElement root, string field) =>
        root.TryGetProperty(field, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() : null;

    private static string Canonical(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        return Canonical(document.RootElement);
    }

    private static string Canonical(JsonElement value) => value.ValueKind switch
    {
        JsonValueKind.Object => "{" + string.Join(",", value.EnumerateObject()
            .OrderBy(property => property.Name, StringComparer.Ordinal)
            .Select(property => JsonSerializer.Serialize(property.Name) + ":" + Canonical(property.Value))) + "}",
        JsonValueKind.Array => "[" + string.Join(",", value.EnumerateArray().Select(Canonical)) + "]",
        JsonValueKind.String => JsonSerializer.Serialize(value.GetString()),
        JsonValueKind.Number or JsonValueKind.True or JsonValueKind.False => value.GetRawText(),
        JsonValueKind.Null => "null",
        _ => throw new InvalidOperationException("unsupported JSON value in canonical comparison")
    };

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
