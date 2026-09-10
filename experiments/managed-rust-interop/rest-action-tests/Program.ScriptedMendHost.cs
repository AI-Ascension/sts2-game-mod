// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private sealed class MendHost : IRuntimeV4ExpertRestHostSource
    {
        private int _phase;
        private readonly bool _captureIdentity;
        private (RuntimeV4ExpertRestOperation Operation,
            RuntimeV4ExpertRestActionReference Action)? _pending;

        internal MendHost(bool captureIdentity = false) => _captureIdentity = captureIdentity;

        private string SelectionId => _captureIdentity
            ? "selection:20:mend" : "selection:script:mend";
        private string PlayerId => _captureIdentity ? "player:local" : "player:2";
        private string PlayerLabel => _captureIdentity ? "Ironclad" : "Target";

        public RuntimeV4ExpertRestHostProjection ObserveRest()
        {
            if (_phase == 0)
            {
                RuntimeV4ExpertRestActionReference option = new(
                    "rest-option:9:mend", new RuntimeV4ExpertRestAction("rest_option", "mend"));
                return new(Observation(9, "rest"), new[] { option });
            }
            if (_phase == 2)
                return new(Observation(22, "rest"), Array.Empty<RuntimeV4ExpertRestActionReference>());
            string selectionId = SelectionId;
            var target = new RuntimeV4ExpertRestActionReference(
                $"select_player:21:mend:{PlayerId}",
                new RuntimeV4ExpertRestAction("select_player", "mend", selectionId,
                    PlayerId: PlayerId));
            var cancel = new RuntimeV4ExpertRestActionReference(
                "cancel_selection:21:mend",
                new RuntimeV4ExpertRestAction("cancel_selection", "mend", selectionId));
            var selector = new RuntimeV4ExpertRestSelector(selectionId, "player", 1,
                Array.Empty<string>(), 1, new[] { target, cancel });
            RuntimeV4ExpertGameplayObservation observation = Observation(21, "selection") with
            {
                State = new RuntimeV4ExpertGameplayState("selection")
                {
                    Choices = new[]
                    {
                        new RuntimeV4ExpertGameplayChoice(
                            PlayerId, PlayerLabel, "selection", null)
                    }
                }
            };
            return new(observation, selector.LegalActions, selector);
        }

        public bool DispatchRest(RuntimeV4ExpertRestOperation operation,
            RuntimeV4ExpertRestActionReference action, RuntimeV4ExpertRestHostProjection current)
        {
            if (_pending is not null || !current.LegalActions.Contains(action)) return false;
            _pending = (operation, action);
            return true;
        }

        public RuntimeV4ExpertRestHostCompletion? CompleteRest(
            RuntimeV4ExpertRestOperation operation, RuntimeV4ExpertRestActionReference action)
        {
            if (_pending is not { } pending || pending.Operation != operation
                || pending.Action != action) return null;
            _pending = null;
            if (_phase == 0 && action.Action.Kind == "rest_option")
            {
                _phase = 1;
                RuntimeV4ExpertRestSelector selector = ObserveRest().Selector!;
                RuntimeV4ExpertGameplayObservation observation = ObserveRest().Observation;
                var transition = new RuntimeV4ExpertRestSelectionRequestedTransition(
                    "mend", 9, 21, selector);
                return new("settled", observation, transition, null, null);
            }
            if (_phase == 1 && action.Action.Kind == "select_player")
            {
                _phase = 2;
                RuntimeV4ExpertGameplayObservation after = Observation(22, "rest") with
                {
                    Player = Observation(22, "rest").Player with { Hp = 74 }
                };
                var witness = new RuntimeV4ExpertRestEffectWitness(
                    "mend_applied", operation, "mend", 22,
                    new RuntimeV4ExpertRestHpEvidence(64, 74, 80, 80), PlayerId);
                var transition = new RuntimeV4ExpertRestSelectionCompletedTransition(
                    "mend", 21, 22, SelectionId, "player", 1,
                    new[] { PlayerId }, witness);
                return new("settled", after, transition, witness, null);
            }
            if (_phase == 1 && action.Action.Kind == "cancel_selection")
            {
                _phase = 2;
                return new("cancelled", null, null, null, "sts2.game-mod/selection_cancelled");
            }
            return null;
        }
    }
}
