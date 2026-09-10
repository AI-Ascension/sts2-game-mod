// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private static void SemanticValidationChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:semantic");
        OptionWitnessMismatchIsRejected(context);
        NoOpHealIsRejected(context);
        SelectorCatalogMustBeActionableAndVisible(context);
        SelectorOptionKindMustMatch(context);
    }

    private static void OptionWitnessMismatchIsRejected(RuntimeV4ExpertRestContext context)
    {
        RuntimeV4ExpertRestOperation operation = SemanticOperation(context, "kind-mismatch");
        RuntimeV4ExpertRestActionReference action = OptionAction("kindle");
        var witness = new RuntimeV4ExpertRestEffectWitness(
            "heal_applied", operation, "kindle", 10,
            new RuntimeV4ExpertRestHpEvidence(64, 70, 80, 80));
        var response = SettledOption(context, operation, action, "kindle", witness);
        Reject(response, "option-to-witness mismatch was accepted");
    }

    private static void NoOpHealIsRejected(RuntimeV4ExpertRestContext context)
    {
        RuntimeV4ExpertRestOperation operation = SemanticOperation(context, "heal-no-op");
        RuntimeV4ExpertRestActionReference action = OptionAction("heal");
        var witness = new RuntimeV4ExpertRestEffectWitness(
            "heal_applied", operation, "heal", 10,
            new RuntimeV4ExpertRestHpEvidence(64, 64, 80, 80));
        var response = SettledOption(context, operation, action, "heal", witness);
        Reject(response, "unchanged HP evidence was accepted");
    }

    private static void SelectorCatalogMustBeActionableAndVisible(
        RuntimeV4ExpertRestContext context)
    {
        RuntimeV4ExpertRestOperation operation = SemanticOperation(context, "selector-catalog");
        RuntimeV4ExpertRestActionReference action = OptionAction("smith");
        var noChoiceSelector = Selector("semantic:no-choice", "card", 2, 2,
            new RuntimeV4ExpertRestActionReference("cancel:semantic:no-choice",
                new RuntimeV4ExpertRestAction("cancel_selection", "smith", "semantic:no-choice")));
        RejectSelector(context, operation, action, noChoiceSelector,
            "selector catalog without a choice action was accepted");

        var invisibleSelector = Selector("semantic:invisible", "card", 1, 1,
            new RuntimeV4ExpertRestActionReference("select:semantic:invisible",
                new RuntimeV4ExpertRestAction("select_card", "smith", "semantic:invisible",
                    CardId: "card:bogus")),
            new RuntimeV4ExpertRestActionReference("cancel:semantic:invisible",
                new RuntimeV4ExpertRestAction("cancel_selection", "smith", "semantic:invisible")));
        RejectSelector(context, operation with { OperationId = "selector-invisible" }, action,
            invisibleSelector, "selector action outside the visible choices was accepted");
    }

    private static void SelectorOptionKindMustMatch(RuntimeV4ExpertRestContext context)
    {
        RuntimeV4ExpertRestOperation operation = SemanticOperation(context, "selector-kind");
        RuntimeV4ExpertRestActionReference action = OptionAction("mend");
        var selector = Selector("semantic:kind", "card", 1, 1,
            new RuntimeV4ExpertRestActionReference("select:semantic:kind",
                new RuntimeV4ExpertRestAction("select_card", "mend", "semantic:kind",
                    CardId: "card:1")),
            new RuntimeV4ExpertRestActionReference("cancel:semantic:kind",
                new RuntimeV4ExpertRestAction("cancel_selection", "mend", "semantic:kind")));
        RejectSelector(context, operation, action, selector,
            "mend selector advertised a card selection");
    }

    private static RuntimeV4ExpertRestSelector Selector(
        string selectionId, string selectionKind, int required, int remaining,
        params RuntimeV4ExpertRestActionReference[] actions) => new(
            selectionId, selectionKind, required, Array.Empty<string>(), remaining, actions);

    private static RuntimeV4ExpertRestOperation SemanticOperation(
        RuntimeV4ExpertRestContext context, string id) => new(
            context.InstanceId, context.SessionId, context.LeaseId, context.LeaseEpoch,
            "rest-op:semantic:" + id);

    private static RuntimeV4ExpertRestActionReference OptionAction(string option) => new(
        "rest-option:semantic:" + option,
        new RuntimeV4ExpertRestAction("rest_option", option));

    private static RuntimeV4ExpertRestResponse SettledOption(
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action,
        string option,
        RuntimeV4ExpertRestEffectWitness witness) => new(
            context, "live:10", 10, operation, action, "settled", Observation(10, "rest"),
            new RuntimeV4ExpertRestCompletedTransition(option, 9, 10, witness), witness, null);

    private static void RejectSelector(
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestSelector selector,
        string message)
    {
        var response = new RuntimeV4ExpertRestResponse(
            context, "live:10", 10, operation, action, "settled", Observation(10, "selection"),
            new RuntimeV4ExpertRestSelectionRequestedTransition(
                action.Action.RestOptionId, 9, 10, selector),
            null, null);
        Reject(response, message);
    }

    private static void Reject(RuntimeV4ExpertRestResponse response, string message) =>
        Check(!RuntimeV4ExpertRestActionCodec.TrySerializeResponse(response, out _, out _), message);
}
