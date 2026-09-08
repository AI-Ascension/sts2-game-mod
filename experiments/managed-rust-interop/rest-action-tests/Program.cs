// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private const string ProposalRelativePath = "docs/proposals/runtime-v4-expert-rest-action-v1";
    private static readonly string[] FirstSelected = { "card:1" };
    private static readonly string[] FirstProgressActions =
        { "select_card:11:smith:card:2", "cancel_selection:11:smith" };
    private static readonly string[] SecondSelected = { "card:1", "card:2" };
    private static readonly string[] SecondProgressActions =
        { "confirm_selection:12:smith", "cancel_selection:12:smith" };
    private static readonly string[] MendSelected = { "player:2" };

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
            NativeWitnessChecks();
            SemanticValidationChecks();
            ProducerConsumerSupportChecks();
            SmithProducerConsumerChecks();
            MendProducerConsumerChecks();
            StaleAndEarlyConfirmationChecks();
            UnknownReconciliationChecks();
            SelectorAdmissionBindingChecks();
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
}
