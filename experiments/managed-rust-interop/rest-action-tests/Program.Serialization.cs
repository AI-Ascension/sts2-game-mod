// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
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

    private static RuntimeV4ExpertGameplayObservation Observation(
        ulong generation, string stateKind = "selection") =>
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
            new RuntimeV4ExpertGameplayState(stateKind)
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

        internal bool ThrowDispatch { get; set; }
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
            if (ThrowDispatch) throw new InvalidOperationException("synthetic host uncertainty");
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
}
