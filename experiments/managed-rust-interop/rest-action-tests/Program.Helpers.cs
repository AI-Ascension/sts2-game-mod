// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
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

    private static RuntimeV4ExpertGameplayObservation SelectorObservation(
        ulong generation,
        bool extendedCatalog,
        bool omitCard2 = false)
    {
        IEnumerable<RuntimeV4ExpertGameplayChoice> choices =
            (Observation(generation, "selection").State.Choices
                ?? Array.Empty<RuntimeV4ExpertGameplayChoice>()).Concat(
                    extendedCatalog ? new[] { new RuntimeV4ExpertGameplayChoice(
                        "card:3", "Defend", "selection", null) }
                    : Array.Empty<RuntimeV4ExpertGameplayChoice>());
        return Observation(generation, "selection") with
        {
            State = Observation(generation, "selection").State with
            {
                Choices = (omitCard2
                    ? choices.Where(choice => choice.ChoiceId != "card:2") : choices).ToArray()
            }
        };
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
