// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private static void SmithProducerConsumerChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:smith-script");
        var host = new SmithHost();
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());

        RuntimeV4ExpertRestHostProjection initial = host.ObserveRest();
        RuntimeV4ExpertRestRequest parent = RequestFromProjection(context, initial,
            "rest-op:smith:parent", initial.LegalActions.Single());
        ExpectAccepted(support, context, parent);
        string requested = Reconcile(support, context, parent.Operation.OperationId);
        CheckTransition(requested, context, "rest_option_selection_requested");

        RuntimeV4ExpertRestHostProjection firstSurface = host.ObserveRest();
        Check(firstSurface.Selector?.RemainingCount == 2,
            "Smith script did not expose its initial remaining count");
        RuntimeV4ExpertRestRequest first = RequestFromProjection(context, firstSurface,
            "rest-op:smith:first", firstSurface.LegalActions.Single(action =>
                action.Action.Kind == "select_card" && action.Action.CardId == "card:1"));
        ExpectAccepted(support, context, first);
        string progressed = Reconcile(support, context, first.Operation.OperationId);
        CheckTransition(progressed, context, "rest_option_selection_progressed");
        Check(JsonDocument.Parse(progressed).RootElement.GetProperty("transition")
            .GetProperty("remaining_count").GetInt32() == 1,
            "Smith first selection did not reduce remaining count");

        RuntimeV4ExpertRestHostProjection secondSurface = host.ObserveRest();
        RuntimeV4ExpertRestRequest second = RequestFromProjection(context, secondSurface,
            "rest-op:smith:second", secondSurface.LegalActions.Single(action =>
                action.Action.Kind == "select_card" && action.Action.CardId == "card:2"));
        ExpectAccepted(support, context, second);
        string ready = Reconcile(support, context, second.Operation.OperationId);
        CheckTransition(ready, context, "rest_option_selection_progressed");
        Check(JsonDocument.Parse(ready).RootElement.GetProperty("transition")
            .GetProperty("remaining_count").GetInt32() == 0,
            "Smith second selection did not expose a complete count");

        RuntimeV4ExpertRestHostProjection confirmSurface = host.ObserveRest();
        RuntimeV4ExpertRestRequest confirmation = RequestFromProjection(context, confirmSurface,
            "rest-op:smith:confirm", confirmSurface.LegalActions.Single(action =>
                action.Action.Kind == "confirm_selection"));
        ExpectAccepted(support, context, confirmation);
        string completed = Reconcile(support, context, confirmation.Operation.OperationId);
        CheckTransition(completed, context, "rest_option_selection_completed");
        using JsonDocument document = JsonDocument.Parse(completed);
        JsonElement transition = document.RootElement.GetProperty("transition");
        Check(transition.GetProperty("selected_choice_ids").GetArrayLength() == 2,
            "Smith completion did not retain both selected cards");
        Check(document.RootElement.GetProperty("effect_witness").GetProperty("kind")
                .GetString() == "smith_applied",
            "Smith completion did not carry its effect witness");
    }

    private static void MendProducerConsumerChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:mend-script");
        var host = new MendHost();
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());
        RuntimeV4ExpertRestHostProjection initial = host.ObserveRest();
        RuntimeV4ExpertRestRequest parent = RequestFromProjection(context, initial,
            "rest-op:mend:parent", initial.LegalActions.Single());
        ExpectAccepted(support, context, parent);
        string requested = Reconcile(support, context, parent.Operation.OperationId);
        CheckTransition(requested, context, "rest_option_selection_requested");

        RuntimeV4ExpertRestHostProjection targetSurface = host.ObserveRest();
        RuntimeV4ExpertRestActionReference targetAction = targetSurface.LegalActions.Single(action =>
            action.Action.Kind == "select_player");
        RuntimeV4ExpertRestRequest target = RequestFromProjection(context, targetSurface,
            "rest-op:mend:target", targetAction);
        ExpectAccepted(support, context, target);
        string completed = Reconcile(support, context, target.Operation.OperationId);
        CheckTransition(completed, context, "rest_option_selection_completed");
        using JsonDocument document = JsonDocument.Parse(completed);
        Check(document.RootElement.GetProperty("effect_witness").GetProperty("target_player_id")
                .GetString() == "player:2",
            "Mend completion lost its selected player identity");

        var cancellationHost = new MendHost();
        RuntimeV4ExpertRestActionSupport cancellationSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(cancellationHost, work => work());
        RuntimeV4ExpertRestHostProjection cancellationInitial = cancellationHost.ObserveRest();
        RuntimeV4ExpertRestRequest cancellationParent = RequestFromProjection(context,
            cancellationInitial, "rest-op:mend:cancel-parent", cancellationInitial.LegalActions.Single());
        ExpectAccepted(cancellationSupport, context, cancellationParent);
        Reconcile(cancellationSupport, context, cancellationParent.Operation.OperationId);
        RuntimeV4ExpertRestHostProjection cancellationSurface = cancellationHost.ObserveRest();
        RuntimeV4ExpertRestRequest cancellation = RequestFromProjection(context, cancellationSurface,
            "rest-op:mend:cancel", cancellationSurface.LegalActions.Single(action =>
                action.Action.Kind == "cancel_selection"));
        ExpectAccepted(cancellationSupport, context, cancellation);
        string cancelled = Reconcile(cancellationSupport, context, cancellation.Operation.OperationId);
        using JsonDocument cancelledDocument = JsonDocument.Parse(cancelled);
        Check(cancelledDocument.RootElement.GetProperty("status").GetString() == "cancelled",
            "Mend cancellation did not return cancelled");
        Check(cancelledDocument.RootElement.GetProperty("effect_witness").ValueKind
                == JsonValueKind.Null, "Mend cancellation carried an effect witness");
    }
}
