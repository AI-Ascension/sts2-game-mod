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
        private readonly bool _captureIdentity;
        private (RuntimeV4ExpertRestOperation Operation,
            RuntimeV4ExpertRestActionReference Action)? _pending;

        internal SmithHost(int initialPhase = 0, bool captureIdentity = false)
        {
            _phase = initialPhase;
            _captureIdentity = captureIdentity;
        }

        internal bool TamperProgression { get; set; }
        internal bool TamperCatalog { get; set; }
        internal bool TamperCancellation { get; set; }
        internal bool TamperDroppedCatalog { get; set; }
        internal bool NativeCatalogChanged { get; set; }

        private bool ExtendedCatalog => TamperDroppedCatalog || NativeCatalogChanged;
        private string SelectionId => _captureIdentity
            ? "selection:10:smith" : "selection:script:smith";

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
            return new(Program.SelectorObservation(generation, ExtendedCatalog,
                NativeCatalogChanged && _phase >= 2), selector.LegalActions, selector);
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
                return new("settled", Program.SelectorObservation(10, ExtendedCatalog),
                    transition, null, null);
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
                RuntimeV4ExpertGameplayObservation observation = Program.SelectorObservation(11,
                    ExtendedCatalog, NativeCatalogChanged);
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
                            Choices = (observation.State.Choices ??
                                Array.Empty<RuntimeV4ExpertGameplayChoice>()).Append(
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
                    "smith", before, 13, SelectionId, "card", 2,
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

        private RuntimeV4ExpertRestSelector Selector(ulong generation, int phase)
        {
            string selectionId = SelectionId;
            var actions = new List<RuntimeV4ExpertRestActionReference>();
            if (phase == 1)
            {
                actions.Add(CardAction(generation, selectionId, "card:1"));
                actions.Add(CardAction(generation, selectionId, "card:2"));
                if (ExtendedCatalog)
                    actions.Add(CardAction(generation, selectionId, "card:3"));
            }
            if (phase == 2)
            {
                actions.Add(CardAction(generation, selectionId, "card:2"));
                if (ExtendedCatalog)
                    actions.Add(CardAction(generation, selectionId, "card:3"));
                if (TamperDroppedCatalog || NativeCatalogChanged)
                    actions.RemoveAll(action => action.Action.CardId == "card:2");
            }
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

}
