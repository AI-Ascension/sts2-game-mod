// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Entities.RestSite;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource : IRuntimeV4ExpertRestHostSource
{
    private readonly Dictionary<RuntimeV4ExpertRestOperation, ExpertRestPending>
        _expertRestPending = new();
    private readonly Dictionary<string, ExpertRestSelector> _expertRestSelectors = new(
        StringComparer.Ordinal);

    /// <summary>
    /// Produces the rest profile's typed catalog from the currently visible native buttons.
    /// Native option IDs are retained on the pending operation; the lower-case wire spelling is
    /// an explicit compatibility mapping for the reviewed profile's existing fixtures.
    /// </summary>
    internal RuntimeV4ExpertRestHostProjection ObserveExpertRest()
    {
        RequireThread();
        RuntimeV4ExpertGameplayObservation observation = ObserveExpert();
        if (_expertRestSelectors.Count > 1)
            return new(observation, Array.Empty<RuntimeV4ExpertRestActionReference>());
        if (_expertRestSelectors.Count == 1)
        {
            ExpertRestSelector selector = _expertRestSelectors.Values.Single();
            return SelectorProjection(observation, selector);
        }

        if (observation.State.Kind != "rest")
            return new(observation, Array.Empty<RuntimeV4ExpertRestActionReference>());
        NRestSiteRoom? room = CurrentRestSite();
        if (room is null)
            return new(observation, Array.Empty<RuntimeV4ExpertRestActionReference>());

        var actions = new List<RuntimeV4ExpertRestActionReference>();
        foreach (NRestSiteButton button in RestButtons(room))
        {
            string? optionId = WireRestOptionId(button.Option.OptionId);
            if (optionId is null || !RuntimeV4ExpertRestActionContract.IsOptionKind(optionId))
                continue;
            if (RestButtons(room).Count(candidate =>
                    WireRestOptionId(candidate.Option.OptionId) == optionId) != 1)
                continue;
            actions.Add(RestOptionAction(observation.Generation, optionId));
        }
        return new(observation, actions);
    }

    bool IRuntimeV4ExpertRestHostSource.DispatchRest(
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestHostProjection current) =>
        DispatchExpertRest(operation, action, current);

    RuntimeV4ExpertRestHostProjection IRuntimeV4ExpertRestHostSource.ObserveRest() =>
        ObserveExpertRest();

    RuntimeV4ExpertRestHostCompletion?
        IRuntimeV4ExpertRestHostSource.CompleteRest(
            RuntimeV4ExpertRestOperation operation,
            RuntimeV4ExpertRestActionReference action) =>
        CompleteExpertRest(operation, action);

    private static RuntimeV4ExpertRestActionReference RestOptionAction(
        ulong generation,
        string optionId) => new(
            $"rest-option:{generation}:{optionId}",
            new RuntimeV4ExpertRestAction("rest_option", optionId));

    private static string? WireRestOptionId(string? nativeId) => nativeId?.ToUpperInvariant() switch
    {
        "CLONE" => "clone",
        "COOK" => "cook",
        "DIG" => "dig",
        "HATCH" => "hatch",
        "HEAL" => "heal",
        "KINDLE" => "kindle",
        "LIFT" => "lift",
        "SMITH" => "smith",
        "MEND" => "mend",
        _ => null
    };

    private bool DispatchExpertRest(
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestHostProjection current)
    {
        RequireThread();
        if (_expertRestPending.ContainsKey(operation)
            || _expertRestPending.Count >= RuntimeV4ExpertRestActionContract.MaxReceipts)
            return false;
        if (!RuntimeV4ExpertRestActionContract.TryValidateAction(action, out _)
            || current.Observation.StateId.Length == 0
            || !current.LegalActions.Contains(action))
            return false;

        if (action.Action.Kind == "rest_option")
        {
            if (_expertRestSelectors.Count != 0 || CurrentRestSite() is not { } room)
                return false;
            string requestedNativeId = action.Action.RestOptionId.ToUpperInvariant();
            NRestSiteButton[] matches = RestButtons(room).Where(button =>
                string.Equals(button.Option.OptionId, requestedNativeId,
                    StringComparison.OrdinalIgnoreCase)).ToArray();
            if (matches.Length != 1 || WireRestOptionId(matches[0].Option.OptionId)
                != action.Action.RestOptionId)
                return false;

            NRestSiteButton button = matches[0];
            RestSiteOption option = button.Option;
            var pending = new ExpertRestPending(operation, action, current.Observation,
                button, option);
            _expertRestPending.Add(operation, pending);
            pending.Work = option switch
            {
                SmithRestSiteOption smith => BeginSmithSelectorAsync(pending, smith),
                MendRestSiteOption mend => BeginMendSelectorAsync(pending, mend),
                _ => ExecuteImmediateRestAsync(pending)
            };
            return true;
        }

        return DispatchSelectorAction(operation, action, current);
    }

    private async Task ExecuteImmediateRestAsync(ExpertRestPending pending)
    {
        pending.Button.ForceClick();
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            NRestSiteRoom? room = CurrentRestSite();
            if (room is null) return;
            NRestSiteButton? current = room.GetButtonForOption(pending.Option);
            if (current is null && Clickable(room.ProceedButton)) return;
            if (current is not null && !ReferenceEquals(current, pending.Button)
                && Clickable(room.ProceedButton)) return;
        }
        throw new InvalidOperationException("native rest option did not settle");
    }

    private RuntimeV4ExpertRestHostCompletion? CompleteExpertRest(
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action)
    {
        RequireThread();
        if (!_expertRestPending.TryGetValue(operation, out ExpertRestPending? pending)
            || pending.Action != action)
            return null;
        if (pending.Completion is not null) return pending.Completion;
        if (pending.Work?.IsCompletedSuccessfully != true) return null;

        if (pending.Selector is not null)
        {
            RuntimeV4ExpertRestHostCompletion? selectorCompletion =
                CompleteSelectorAction(pending);
            if (selectorCompletion is not null) pending.Completion = selectorCompletion;
            return selectorCompletion;
        }

        RuntimeV4ExpertGameplayObservation after = ObserveExpert();
        if (after.Generation <= pending.Before.Generation) return null;
        RuntimeV4ExpertRestEffectWitness? witness = ImmediateWitness(
            pending.Operation, pending.Option, pending.Before, after);
        if (witness is null) return null;
        var transition = new RuntimeV4ExpertRestCompletedTransition(
            WireRestOptionId(pending.Option.OptionId)!, pending.Before.Generation,
            after.Generation, witness);
        var completion = new RuntimeV4ExpertRestHostCompletion(
            "settled", after, transition, witness, null);
        _expertRestPending.Remove(operation);
        pending.Completion = completion;
        return completion;
    }

    private static RuntimeV4ExpertRestEffectWitness? ImmediateWitness(
        RuntimeV4ExpertRestOperation operation,
        RestSiteOption option,
        RuntimeV4ExpertGameplayObservation before,
        RuntimeV4ExpertGameplayObservation after)
    {
        string optionId = WireRestOptionId(option.OptionId)!;
        RuntimeV4ExpertRestEvidence? evidence = optionId switch
        {
            "heal" => before.Player.Hp <= after.Player.Hp
                ? new RuntimeV4ExpertRestHpEvidence(before.Player.Hp, after.Player.Hp,
                    before.Player.MaxHp, after.Player.MaxHp)
                : null,
            "clone" => CardEvidence(before.Player.Deck, after.Player.Deck,
                requireAdded: true, requireRemoved: false, requireUpgraded: false),
            "cook" => CardEvidence(before.Player.Deck, after.Player.Deck,
                requireAdded: false, requireRemoved: true, requireUpgraded: false),
            "dig" or "hatch" => RelicEvidence(before.Player.Relics, after.Player.Relics),
            "kindle" or "lift" => new RuntimeV4ExpertRestNativeEvidence(
                $"{optionId}:{operation.OperationId}", after.StateId),
            _ => null
        };
        if (evidence is null) return null;
        return new RuntimeV4ExpertRestEffectWitness(
            optionId + "_applied", operation, optionId, after.Generation, evidence);
    }

    private static RuntimeV4ExpertRestCardEvidence? CardEvidence(
        IReadOnlyList<RuntimeV4ExpertGameplayCard>? before,
        IReadOnlyList<RuntimeV4ExpertGameplayCard>? after,
        bool requireAdded,
        bool requireRemoved,
        bool requireUpgraded)
    {
        if (before is null || after is null) return null;
        var beforeById = before.ToDictionary(card => card.CardId, StringComparer.Ordinal);
        var afterById = after.ToDictionary(card => card.CardId, StringComparer.Ordinal);
        string[] added = afterById.Keys.Except(beforeById.Keys, StringComparer.Ordinal).ToArray();
        string[] removed = beforeById.Keys.Except(afterById.Keys, StringComparer.Ordinal).ToArray();
        string[] upgraded = beforeById.Keys.Intersect(afterById.Keys, StringComparer.Ordinal)
            .Where(id => !beforeById[id].Upgraded && afterById[id].Upgraded).ToArray();
        if (requireAdded && added.Length == 0 || requireRemoved && removed.Length == 0
            || requireUpgraded && upgraded.Length == 0)
            return null;
        return new RuntimeV4ExpertRestCardEvidence(added, removed, upgraded);
    }

    private static RuntimeV4ExpertRestRelicEvidence? RelicEvidence(
        IReadOnlyList<RuntimeV4ExpertGameplayRelic>? before,
        IReadOnlyList<RuntimeV4ExpertGameplayRelic>? after)
    {
        if (before is null || after is null) return null;
        string[] beforeIds = before.Select(relic => relic.RelicId).ToArray();
        string[] afterIds = after.Select(relic => relic.RelicId).ToArray();
        string[] added = afterIds.Except(beforeIds, StringComparer.Ordinal).ToArray();
        string[] removed = beforeIds.Except(afterIds, StringComparer.Ordinal).ToArray();
        return added.Length == 0
            ? null : new RuntimeV4ExpertRestRelicEvidence(added, removed);
    }

    private sealed class ExpertRestPending
    {
        internal ExpertRestPending(
            RuntimeV4ExpertRestOperation operation,
            RuntimeV4ExpertRestActionReference action,
            RuntimeV4ExpertGameplayObservation before,
            NRestSiteButton button,
            RestSiteOption option)
        {
            Operation = operation;
            Action = action;
            Before = before;
            Button = button;
            Option = option;
        }

        internal RuntimeV4ExpertRestOperation Operation { get; }
        internal RuntimeV4ExpertRestActionReference Action { get; }
        internal RuntimeV4ExpertGameplayObservation Before { get; }
        internal NRestSiteButton Button { get; }
        internal RestSiteOption Option { get; }
        internal ExpertRestSelector? Selector { get; set; }
        internal Task? Work { get; set; }
        internal RuntimeV4ExpertRestHostCompletion? Completion { get; set; }
    }
}
