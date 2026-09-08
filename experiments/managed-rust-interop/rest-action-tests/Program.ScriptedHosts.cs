// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private sealed class SmithHost : IRuntimeV4ExpertRestHostSource
    {
        private int _phase;
        private (RuntimeV4ExpertRestOperation Operation,
            RuntimeV4ExpertRestActionReference Action)? _pending;

        internal SmithHost(int initialPhase = 0)
        {
            _phase = initialPhase;
        }

        internal bool TamperProgression { get; set; }
        internal bool TamperCatalog { get; set; }
        internal bool TamperCancellation { get; set; }

        public RuntimeV4ExpertRestHostProjection ObserveRest()
        {
            ulong generation = (ulong)(9 + _phase);
            if (_phase == 0)
            {
                RuntimeV4ExpertRestActionReference option = new(
                    "rest-option:9:smith", new RuntimeV4ExpertRestAction("rest_option", "smith"));
                return new(Observation(generation, "rest"), new[] { option });
            }
            if (_phase >= 4)
                return new(Observation(13, "rest"), Array.Empty<RuntimeV4ExpertRestActionReference>());
            RuntimeV4ExpertRestSelector selector = Selector(generation, _phase);
            return new(Observation(generation, "selection"), selector.LegalActions, selector);
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
            ulong before = (ulong)(9 + _phase);
            if (_phase == 0 && action.Action.Kind == "rest_option")
            {
                _phase = 1;
                RuntimeV4ExpertRestSelector selector = Selector(10, 1);
                var transition = new RuntimeV4ExpertRestSelectionRequestedTransition(
                    "smith", 9, 10, selector);
                return new("settled", Observation(10, "selection"), transition, null, null);
            }
            if (_phase == 1 && action.Action.Kind == "select_card"
                && action.Action.CardId == "card:1")
            {
                if (TamperCancellation)
                    return new("cancelled", null, null, null, "sts2.game-mod/selection_cancelled");
                _phase = 2;
                RuntimeV4ExpertRestSelector selector = Selector(11, 2);
                if (TamperProgression)
                    selector = selector with { SelectedChoiceIds = new[] { "card:2" } };
                RuntimeV4ExpertGameplayObservation observation = Observation(11, "selection");
                if (TamperCatalog)
                {
                    selector = selector with
                    {
                        LegalActions = selector.LegalActions.Append(CardAction(
                            11, "selection:script:smith", "card:3")).ToArray()
                    };
                    observation = observation with
                    {
                        State = observation.State with
                        {
                            Choices = observation.State.Choices.Append(
                                new RuntimeV4ExpertGameplayChoice(
                                    "card:3", "Defend", "selection", null)).ToArray()
                        }
                    };
                }
                var transition = new RuntimeV4ExpertRestSelectionProgressedTransition(
                    "smith", 10, 11, selector);
                return new("settled", observation, transition, null, null);
            }
            if (_phase == 2 && action.Action.Kind == "select_card"
                && action.Action.CardId == "card:2")
            {
                _phase = 3;
                RuntimeV4ExpertRestSelector selector = Selector(12, 3);
                var transition = new RuntimeV4ExpertRestSelectionProgressedTransition(
                    "smith", 11, 12, selector);
                return new("settled", Observation(12, "selection"), transition, null, null);
            }
            if (_phase == 3 && action.Action.Kind == "confirm_selection")
            {
                _phase = 4;
                var witness = new RuntimeV4ExpertRestEffectWitness(
                    "smith_applied", operation, "smith", 13,
                    new RuntimeV4ExpertRestCardEvidence(Array.Empty<string>(), Array.Empty<string>(),
                        SecondSelected));
                var transition = new RuntimeV4ExpertRestSelectionCompletedTransition(
                    "smith", before, 13, "selection:script:smith", "card", 2,
                    SecondSelected, witness);
                return new("settled", Observation(13, "rest"), transition, witness, null);
            }
            if (_phase is 1 or 2 or 3 && action.Action.Kind == "cancel_selection")
            {
                _phase = 4;
                return new("cancelled", null, null, null, "sts2.game-mod/selection_cancelled");
            }
            return null;
        }

        private static RuntimeV4ExpertRestSelector Selector(ulong generation, int phase)
        {
            string selectionId = "selection:script:smith";
            var actions = new List<RuntimeV4ExpertRestActionReference>();
            if (phase == 1)
            {
                actions.Add(CardAction(generation, selectionId, "card:1"));
                actions.Add(CardAction(generation, selectionId, "card:2"));
            }
            if (phase == 2)
                actions.Add(CardAction(generation, selectionId, "card:2"));
            if (phase == 3)
                actions.Add(new RuntimeV4ExpertRestActionReference(
                    $"confirm_selection:{generation}:smith",
                    new RuntimeV4ExpertRestAction("confirm_selection", "smith", selectionId)));
            actions.Add(new RuntimeV4ExpertRestActionReference(
                $"cancel_selection:{generation}:smith",
                new RuntimeV4ExpertRestAction("cancel_selection", "smith", selectionId)));
            string[] selected = phase switch
            {
                1 => Array.Empty<string>(),
                2 => new[] { "card:1" },
                _ => new[] { "card:1", "card:2" }
            };
            return new(selectionId, "card", 2, selected, 2 - selected.Length, actions);
        }

        private static RuntimeV4ExpertRestActionReference CardAction(
            ulong generation, string selectionId, string cardId) => new(
                $"select_card:{generation}:smith:{cardId}",
                new RuntimeV4ExpertRestAction("select_card", "smith", selectionId, cardId));
    }

    private sealed class MendHost : IRuntimeV4ExpertRestHostSource
    {
        private int _phase;
        private (RuntimeV4ExpertRestOperation Operation,
            RuntimeV4ExpertRestActionReference Action)? _pending;

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
            string selectionId = "selection:script:mend";
            var target = new RuntimeV4ExpertRestActionReference(
                "select_player:21:mend:player:2",
                new RuntimeV4ExpertRestAction("select_player", "mend", selectionId,
                    PlayerId: "player:2"));
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
                            "player:2", "Target", "selection", null)
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
                    new RuntimeV4ExpertRestHpEvidence(64, 74, 80, 80), "player:2");
                var transition = new RuntimeV4ExpertRestSelectionCompletedTransition(
                    "mend", 21, 22, "selection:script:mend", "player", 1,
                    MendSelected, witness);
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
