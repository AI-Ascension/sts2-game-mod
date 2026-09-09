// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private static void StaleAndEarlyConfirmationChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:stale-script");
        RuntimeV4ExpertRestOperation currentOperation = new(
            context.InstanceId, context.SessionId, context.LeaseId, context.LeaseEpoch,
            "rest-op:stale:current");
        RuntimeV4ExpertRestActionReference currentAction = new(
            "rest-option:9:heal", new RuntimeV4ExpertRestAction("rest_option", "heal"));
        RuntimeV4ExpertRestRequest current = new(context, currentOperation, currentAction,
            "live:9", 9);
        var staleHost = new FakeRestHost(current);
        RuntimeV4ExpertRestActionSupport staleSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(staleHost, work => work());
        RuntimeV4ExpertRestRequest stale = current with
        {
            Operation = currentOperation with { OperationId = "rest-op:stale:old" },
            StateId = "live:8", Generation = 8
        };
        string staleResponse = Submit(staleSupport, context, stale);
        using (JsonDocument staleDocument = JsonDocument.Parse(staleResponse))
        {
            Check(staleDocument.RootElement.GetProperty("status").GetString() == "rejected",
                "stale rest generation was not rejected");
            Check(staleDocument.RootElement.GetProperty("error_code").GetString()
                    == "sts2.game-mod/rest_option_not_legal",
                "stale rest generation returned the wrong error");
        }

        var earlyHost = new SmithHost(initialPhase: 2);
        RuntimeV4ExpertRestActionSupport earlySupport =
            RuntimeV4ExpertRestActionSupport.WithHost(earlyHost, work => work());
        RuntimeV4ExpertRestHostProjection earlySurface = earlyHost.ObserveRest();
        RuntimeV4ExpertRestActionReference earlyAction = new(
            "confirm_selection:11:smith",
            new RuntimeV4ExpertRestAction("confirm_selection", "smith",
                "selection:script:smith"));
        RuntimeV4ExpertRestRequest early = RequestFromProjection(context, earlySurface,
            "rest-op:early-confirm", earlyAction);
        string earlyResponse = Submit(earlySupport, context, early);
        using JsonDocument earlyDocument = JsonDocument.Parse(earlyResponse);
        Check(earlyDocument.RootElement.GetProperty("status").GetString() == "rejected",
            "early Smith confirmation was not rejected by the executable producer");
        Check(earlyDocument.RootElement.GetProperty("error_code").GetString()
                == "sts2.game-mod/rest_option_not_legal",
            "early Smith confirmation returned the wrong executable error");

        var staleSelectorHost = new SmithHost(initialPhase: 3);
        RuntimeV4ExpertRestActionSupport staleSelectorSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(staleSelectorHost, work => work());
        RuntimeV4ExpertRestHostProjection staleSelectorSurface = staleSelectorHost.ObserveRest();
        RuntimeV4ExpertRestActionReference staleSelectorAction = new(
            "select_card:11:smith:card:2",
            new RuntimeV4ExpertRestAction("select_card", "smith",
                "selection:script:smith", CardId: "card:2"));
        RuntimeV4ExpertRestRequest staleSelector = RequestFromProjection(context,
            staleSelectorSurface, "rest-op:stale-selector", staleSelectorAction);
        staleSelector = staleSelector with { StateId = "live:11", Generation = 11 };
        string staleSelectorResponse = Submit(staleSelectorSupport, context, staleSelector);
        using JsonDocument staleSelectorDocument = JsonDocument.Parse(staleSelectorResponse);
        Check(staleSelectorDocument.RootElement.GetProperty("status").GetString() == "rejected",
            "stale selector generation was not rejected");
    }

    private static void UnknownReconciliationChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:unknown-script");
        RuntimeV4ExpertRestOperation operation = new(
            context.InstanceId, context.SessionId, context.LeaseId, context.LeaseEpoch,
            "rest-op:unknown:heal");
        RuntimeV4ExpertRestActionReference action = new(
            "rest-option:9:heal", new RuntimeV4ExpertRestAction("rest_option", "heal"));
        RuntimeV4ExpertRestRequest request = new(context, operation, action, "live:9", 9);
        var host = new FakeRestHost(request) { ThrowDispatch = true };
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());
        string unknown = Submit(support, context, request);
        using (JsonDocument unknownDocument = JsonDocument.Parse(unknown))
            Check(unknownDocument.RootElement.GetProperty("status").GetString() == "unknown",
                "uncertain native dispatch did not retain unknown status");
        string settled = Reconcile(support, context, operation.OperationId);
        using JsonDocument settledDocument = JsonDocument.Parse(settled);
        Check(settledDocument.RootElement.GetProperty("status").GetString() == "settled",
            "same operation reconciliation did not settle the retained unknown");
        Check(host.DispatchCount == 1,
            "unknown reconciliation dispatched the native mutation a second time");

        RuntimeV4ExpertRestOperation missingOperation = operation with
        {
            OperationId = "rest-op:unknown:missing-witness"
        };
        RuntimeV4ExpertRestRequest missingRequest = request with { Operation = missingOperation };
        var missingHost = new FakeRestHost(missingRequest) { MissingCompletion = true };
        RuntimeV4ExpertRestActionSupport missingSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(missingHost, work => work());
        ExpectAccepted(missingSupport, context, missingRequest);
        string missing = Reconcile(missingSupport, context, missingOperation.OperationId);
        using (JsonDocument missingDocument = JsonDocument.Parse(missing))
            Check(missingDocument.RootElement.GetProperty("status").GetString() == "unknown",
                "missing native evidence did not retain unknown status");
        string retried = Reconcile(missingSupport, context, missingOperation.OperationId);
        using (JsonDocument retriedDocument = JsonDocument.Parse(retried))
            Check(retriedDocument.RootElement.GetProperty("status").GetString() == "unknown",
                "missing native evidence did not retain the original operation");
        Check(missingHost.DispatchCount == 1 && missingHost.CompletionCount == 2,
            "missing native evidence retried dispatch instead of reconciling the same operation");
    }

    private static void SelectorAdmissionBindingChecks()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:selector-binding");
        var host = new SmithHost { TamperProgression = true };
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());

        RuntimeV4ExpertRestHostProjection initial = host.ObserveRest();
        RuntimeV4ExpertRestRequest parent = RequestFromProjection(context, initial,
            "rest-op:selector-binding:parent", initial.LegalActions.Single());
        ExpectAccepted(support, context, parent);
        _ = Reconcile(support, context, parent.Operation.OperationId);

        RuntimeV4ExpertRestHostProjection selector = host.ObserveRest();
        RuntimeV4ExpertRestActionReference firstAction = selector.LegalActions.Single(action =>
            action.Action.Kind == "select_card" && action.Action.CardId == "card:1");
        RuntimeV4ExpertRestRequest first = RequestFromProjection(context, selector,
            "rest-op:selector-binding:first", firstAction);
        ExpectAccepted(support, context, first);
        string unknown = Reconcile(support, context, first.Operation.OperationId);
        using JsonDocument document = JsonDocument.Parse(unknown);
        Check(document.RootElement.GetProperty("status").GetString() == "unknown",
            "selector completion with a fabricated choice did not remain unknown");
        JsonElement returnedAction = document.RootElement.GetProperty("action");
        Check(returnedAction.GetProperty("action_id").GetString() == firstAction.ActionId
                && returnedAction.GetProperty("action").GetProperty("kind").GetString()
                    == firstAction.Action.Kind
                && returnedAction.GetProperty("action").GetProperty("rest_option_id").GetString()
                    == firstAction.Action.RestOptionId
                && returnedAction.GetProperty("action").GetProperty("selection_id").GetString()
                    == firstAction.Action.SelectionId
                && returnedAction.GetProperty("action").GetProperty("card_id").GetString()
                    == firstAction.Action.CardId,
            "unknown selector receipt did not retain the admitted action reference");

        var catalogHost = new SmithHost { TamperCatalog = true };
        RuntimeV4ExpertRestActionSupport catalogSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(catalogHost, work => work());
        RuntimeV4ExpertRestHostProjection catalogInitial = catalogHost.ObserveRest();
        RuntimeV4ExpertRestRequest catalogParent = RequestFromProjection(context, catalogInitial,
            "rest-op:selector-binding:catalog-parent", catalogInitial.LegalActions.Single());
        ExpectAccepted(catalogSupport, context, catalogParent);
        _ = Reconcile(catalogSupport, context, catalogParent.Operation.OperationId);
        RuntimeV4ExpertRestHostProjection catalogSelector = catalogHost.ObserveRest();
        RuntimeV4ExpertRestActionReference catalogFirstAction = catalogSelector.LegalActions
            .Single(action => action.Action.Kind == "select_card"
                && action.Action.CardId == "card:1");
        RuntimeV4ExpertRestRequest catalogFirst = RequestFromProjection(context, catalogSelector,
            "rest-op:selector-binding:catalog-first", catalogFirstAction);
        ExpectAccepted(catalogSupport, context, catalogFirst);
        string catalogUnknown = Reconcile(catalogSupport, context,
            catalogFirst.Operation.OperationId);
        using JsonDocument catalogDocument = JsonDocument.Parse(catalogUnknown);
        Check(catalogDocument.RootElement.GetProperty("status").GetString() == "unknown",
            "selector completion with a fabricated follow-up catalog did not remain unknown");

        var droppedCatalogHost = new SmithHost { TamperDroppedCatalog = true };
        RuntimeV4ExpertRestActionSupport droppedCatalogSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(droppedCatalogHost, work => work());
        RuntimeV4ExpertRestHostProjection droppedInitial = droppedCatalogHost.ObserveRest();
        RuntimeV4ExpertRestRequest droppedParent = RequestFromProjection(context, droppedInitial,
            "rest-op:selector-binding:dropped-parent", droppedInitial.LegalActions.Single());
        ExpectAccepted(droppedCatalogSupport, context, droppedParent);
        _ = Reconcile(droppedCatalogSupport, context, droppedParent.Operation.OperationId);
        RuntimeV4ExpertRestHostProjection droppedSelector = droppedCatalogHost.ObserveRest();
        RuntimeV4ExpertRestActionReference droppedFirstAction = droppedSelector.LegalActions
            .Single(action => action.Action.Kind == "select_card"
                && action.Action.CardId == "card:1");
        RuntimeV4ExpertRestRequest droppedFirst = RequestFromProjection(context, droppedSelector,
            "rest-op:selector-binding:dropped-first", droppedFirstAction);
        ExpectAccepted(droppedCatalogSupport, context, droppedFirst);
        string droppedUnknown = Reconcile(droppedCatalogSupport, context,
            droppedFirst.Operation.OperationId);
        using JsonDocument droppedDocument = JsonDocument.Parse(droppedUnknown);
        Check(droppedDocument.RootElement.GetProperty("status").GetString() == "unknown",
            "selector completion that dropped a still-visible choice did not remain unknown");

        var changedCatalogHost = new SmithHost { NativeCatalogChanged = true };
        RuntimeV4ExpertRestActionSupport changedCatalogSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(changedCatalogHost, work => work());
        RuntimeV4ExpertRestHostProjection changedInitial = changedCatalogHost.ObserveRest();
        RuntimeV4ExpertRestRequest changedParent = RequestFromProjection(context, changedInitial,
            "rest-op:selector-binding:changed-parent", changedInitial.LegalActions.Single());
        ExpectAccepted(changedCatalogSupport, context, changedParent);
        _ = Reconcile(changedCatalogSupport, context, changedParent.Operation.OperationId);
        RuntimeV4ExpertRestHostProjection changedSelector = changedCatalogHost.ObserveRest();
        RuntimeV4ExpertRestActionReference changedFirstAction = changedSelector.LegalActions
            .Single(action => action.Action.Kind == "select_card"
                && action.Action.CardId == "card:1");
        RuntimeV4ExpertRestRequest changedFirst = RequestFromProjection(context, changedSelector,
            "rest-op:selector-binding:changed-first", changedFirstAction);
        ExpectAccepted(changedCatalogSupport, context, changedFirst);
        string changedSettled = Reconcile(changedCatalogSupport, context,
            changedFirst.Operation.OperationId);
        using JsonDocument changedDocument = JsonDocument.Parse(changedSettled);
        Check(changedDocument.RootElement.GetProperty("status").GetString() == "settled"
                && changedDocument.RootElement.GetProperty("transition").GetProperty("kind")
                    .GetString() == "rest_option_selection_progressed",
            "selector completion with a native choice removal did not settle its valid catalog");

        var cancellationHost = new SmithHost { TamperCancellation = true };
        RuntimeV4ExpertRestActionSupport cancellationSupport =
            RuntimeV4ExpertRestActionSupport.WithHost(cancellationHost, work => work());
        RuntimeV4ExpertRestHostProjection cancellationInitial = cancellationHost.ObserveRest();
        RuntimeV4ExpertRestRequest cancellationParent = RequestFromProjection(context,
            cancellationInitial, "rest-op:selector-binding:cancel-parent",
            cancellationInitial.LegalActions.Single());
        ExpectAccepted(cancellationSupport, context, cancellationParent);
        _ = Reconcile(cancellationSupport, context, cancellationParent.Operation.OperationId);
        RuntimeV4ExpertRestHostProjection cancellationSelector = cancellationHost.ObserveRest();
        RuntimeV4ExpertRestActionReference cancellationAction = cancellationSelector.LegalActions
            .Single(action => action.Action.Kind == "select_card"
                && action.Action.CardId == "card:1");
        RuntimeV4ExpertRestRequest cancellation = RequestFromProjection(context,
            cancellationSelector, "rest-op:selector-binding:cancel-first", cancellationAction);
        ExpectAccepted(cancellationSupport, context, cancellation);
        string cancellationUnknown = Reconcile(cancellationSupport, context,
            cancellation.Operation.OperationId);
        using JsonDocument cancellationDocument = JsonDocument.Parse(cancellationUnknown);
        Check(cancellationDocument.RootElement.GetProperty("status").GetString() == "unknown",
            "non-cancel selector action reported cancelled status");
    }

    private static RuntimeV4ExpertRestRequest RequestFromProjection(
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestHostProjection projection,
        string operationId,
        RuntimeV4ExpertRestActionReference action) => new(
            context,
            new RuntimeV4ExpertRestOperation(context.InstanceId, context.SessionId,
                context.LeaseId, context.LeaseEpoch, operationId),
            action, projection.Observation.StateId, projection.Observation.Generation);

    private static string Submit(
        RuntimeV4ExpertRestActionSupport support,
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestRequest request)
    {
        Check(RuntimeV4ExpertRestActionCodec.TrySerializeRequest(request, out string body,
                out string error), "script request failed to serialize: " + error);
        (int Status, string Response) response = support.Handle(context, body, out int status);
        Check(response.Status == status, "support tuple and out status disagree");
        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(response.Response, context,
                out error), "script response failed consumer validation: " + error);
        return response.Response;
    }

    private static void ExpectAccepted(
        RuntimeV4ExpertRestActionSupport support,
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestRequest request)
    {
        string response = Submit(support, context, request);
        using JsonDocument document = JsonDocument.Parse(response);
        Check(document.RootElement.GetProperty("status").GetString() == "accepted",
            "script mutation was not accepted");
    }

    private static string Reconcile(
        RuntimeV4ExpertRestActionSupport support,
        RuntimeV4ExpertRestContext context,
        string operationId)
    {
        (int Status, string Response) response = support.Handle(context, operationId,
            out int status);
        Check(response.Status == status, "reconciliation tuple and out status disagree");
        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(response.Response, context,
                out string error), "reconciliation response failed consumer validation: " + error);
        return response.Response;
    }

    private static void CheckTransition(
        string response,
        RuntimeV4ExpertRestContext context,
        string expectedKind)
    {
        using JsonDocument document = JsonDocument.Parse(response);
        Check(document.RootElement.GetProperty("status").GetString() == "settled",
            "script response did not settle");
        Check(document.RootElement.GetProperty("transition").GetProperty("kind").GetString()
                == expectedKind, "script response returned an unexpected transition");
    }
}
