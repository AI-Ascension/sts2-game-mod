// SPDX-License-Identifier: MIT

using System;
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
