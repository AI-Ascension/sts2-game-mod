// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Constants and transport-neutral values for the source-only rest-action candidate.</summary>
internal static class RuntimeV4ExpertRestActionContract
{
    internal const string ProtocolVersion = "runtime-v4-expert-rest-action-v1";
    internal const string Artifact = "sts2-protocol/runtime-v4-expert-rest-action";
    internal const string SchemaSource = "schemas/runtime-v4-expert-rest-action-v1.schema.json";
    internal const string Generator = "hand-authored";
    internal const string Profile = "expert-rest-action";
    internal const string EffectWitnessVersion = "rest-effect-witness-v1";
    internal const int MaxRequestBytes = 128 * 1024;
    internal const int MaxReceipts = 4096;
    internal const int MaxChoices = 256;
    internal const int MaxIdentityLength = 512;

    // This value is updated only when the candidate schema is changed. The source-only codec test
    // compares it with the exact proposal bytes and the copied protocol candidate artifact.
    internal const string SchemaDigest =
        "bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd";

    internal static readonly IReadOnlySet<string> ImmediateOptionKinds =
        new HashSet<string>(StringComparer.Ordinal)
        {
            "clone", "cook", "dig", "hatch", "heal", "kindle", "lift"
        };

    internal static readonly IReadOnlySet<string> SelectorOptionKinds =
        new HashSet<string>(StringComparer.Ordinal) { "smith", "mend" };

    internal static bool IsIdentity(string? value) => value is not null
        && value.Length is > 0 and <= MaxIdentityLength
        && RuntimeV3GameplayContract.IsIdentity(value);

    internal static bool IsOptionKind(string? value) => value is not null
        && (ImmediateOptionKinds.Contains(value) || SelectorOptionKinds.Contains(value));

    internal static bool IsSelectionKind(string? value) => value is "card" or "player";

    internal static bool IsActionKind(string? value) => value is
        "rest_option" or "select_card" or "select_player"
        or "confirm_selection" or "cancel_selection";

    internal static bool TryValidateAction(
        RuntimeV4ExpertRestActionReference? reference, out string error)
    {
        error = string.Empty;
        if (reference is null || !IsIdentity(reference.ActionId))
        {
            error = "rest action reference identity is invalid";
            return false;
        }

        RuntimeV4ExpertRestAction action = reference.Action;
        if (!IsActionKind(action.Kind) || !IsIdentity(action.RestOptionId))
        {
            error = "rest action kind or option identity is invalid";
            return false;
        }

        bool selector = action.Kind is "select_card" or "select_player"
            or "confirm_selection" or "cancel_selection";
        if (!selector && !ImmediateOptionKinds.Contains(action.RestOptionId)
            && !SelectorOptionKinds.Contains(action.RestOptionId))
        {
            error = "rest action option is outside the audited native set";
            return false;
        }
        if (selector && !IsIdentity(action.SelectionId))
        {
            error = "selector action has no selection identity";
            return false;
        }
        if (action.Kind == "select_card" && !IsIdentity(action.CardId))
        {
            error = "card selector action has no card identity";
            return false;
        }
        if (action.Kind == "select_player" && !IsIdentity(action.PlayerId))
        {
            error = "player selector action has no player identity";
            return false;
        }
        if (action.Kind is "confirm_selection" or "cancel_selection"
            && (action.CardId is not null || action.PlayerId is not null))
        {
            error = "selector control action has a choice identity";
            return false;
        }
        if (action.Kind == "select_card" && action.PlayerId is not null
            || action.Kind == "select_player" && action.CardId is not null)
        {
            error = "selector action has the wrong choice identity";
            return false;
        }
        if (action.Kind == "rest_option" && (action.SelectionId is not null
            || action.CardId is not null || action.PlayerId is not null))
        {
            error = "rest option action has selector fields";
            return false;
        }
        if (action.Kind == "select_card" && action.RestOptionId != "smith"
            || action.Kind == "select_player" && action.RestOptionId != "mend"
            || (action.Kind is "confirm_selection" or "cancel_selection")
                && !SelectorOptionKinds.Contains(action.RestOptionId))
        {
            error = "selector action does not match its native option";
            return false;
        }
        return true;
    }
}

internal sealed record RuntimeV4ExpertRestContext(
    string InstanceId,
    string SessionId,
    string LeaseId,
    ulong LeaseEpoch,
    string CorrelationId);

internal sealed record RuntimeV4ExpertRestOperation(
    string InstanceId,
    string SessionId,
    string LeaseId,
    ulong LeaseEpoch,
    string OperationId);

internal sealed record RuntimeV4ExpertRestAction(
    string Kind,
    string RestOptionId,
    string? SelectionId = null,
    string? CardId = null,
    string? PlayerId = null);

internal sealed record RuntimeV4ExpertRestActionReference(
    string ActionId,
    RuntimeV4ExpertRestAction Action);

internal sealed record RuntimeV4ExpertRestRequest(
    RuntimeV4ExpertRestContext Context,
    RuntimeV4ExpertRestOperation Operation,
    RuntimeV4ExpertRestActionReference Action,
    string StateId,
    ulong Generation);

internal abstract record RuntimeV4ExpertRestEvidence(string Kind);

internal sealed record RuntimeV4ExpertRestHpEvidence(
    ushort HpBefore,
    ushort HpAfter,
    ushort MaxHpBefore,
    ushort MaxHpAfter) : RuntimeV4ExpertRestEvidence("hp_change");

internal sealed record RuntimeV4ExpertRestCardEvidence(
    IReadOnlyList<string> AddedCardIds,
    IReadOnlyList<string> RemovedCardIds,
    IReadOnlyList<string> UpgradedCardIds) : RuntimeV4ExpertRestEvidence("card_change");

internal sealed record RuntimeV4ExpertRestRelicEvidence(
    IReadOnlyList<string> AddedRelicIds,
    IReadOnlyList<string> RemovedRelicIds) : RuntimeV4ExpertRestEvidence("relic_change");

internal sealed record RuntimeV4ExpertRestStatEvidence(
    string StatId,
    int Before,
    int After) : RuntimeV4ExpertRestEvidence("stat_change");

internal sealed record RuntimeV4ExpertRestNativeEvidence(
    string CompletionId,
    string NativeStateId) : RuntimeV4ExpertRestEvidence("native_completion");

internal sealed record RuntimeV4ExpertRestEffectWitness(
    string Kind,
    RuntimeV4ExpertRestOperation Operation,
    string RestOptionId,
    ulong Generation,
    RuntimeV4ExpertRestEvidence Evidence,
    string? TargetPlayerId = null);

internal sealed record RuntimeV4ExpertRestSelector(
    string SelectionId,
    string SelectionKind,
    int RequiredCount,
    IReadOnlyList<string> SelectedChoiceIds,
    int RemainingCount,
    IReadOnlyList<RuntimeV4ExpertRestActionReference> LegalActions);

internal abstract record RuntimeV4ExpertRestTransition(
    string RestOptionId,
    ulong BeforeGeneration,
    ulong AfterGeneration);

internal sealed record RuntimeV4ExpertRestCompletedTransition(
    string RestOptionId,
    ulong BeforeGeneration,
    ulong AfterGeneration,
    RuntimeV4ExpertRestEffectWitness EffectWitness)
    : RuntimeV4ExpertRestTransition(RestOptionId, BeforeGeneration, AfterGeneration);

internal sealed record RuntimeV4ExpertRestSelectionRequestedTransition(
    string RestOptionId,
    ulong BeforeGeneration,
    ulong AfterGeneration,
    RuntimeV4ExpertRestSelector Selector)
    : RuntimeV4ExpertRestTransition(RestOptionId, BeforeGeneration, AfterGeneration);

internal sealed record RuntimeV4ExpertRestSelectionProgressedTransition(
    string RestOptionId,
    ulong BeforeGeneration,
    ulong AfterGeneration,
    RuntimeV4ExpertRestSelector Selector)
    : RuntimeV4ExpertRestTransition(RestOptionId, BeforeGeneration, AfterGeneration);

internal sealed record RuntimeV4ExpertRestSelectionCompletedTransition(
    string RestOptionId,
    ulong BeforeGeneration,
    ulong AfterGeneration,
    string SelectionId,
    string SelectionKind,
    int RequiredCount,
    IReadOnlyList<string> SelectedChoiceIds,
    RuntimeV4ExpertRestEffectWitness EffectWitness)
    : RuntimeV4ExpertRestTransition(RestOptionId, BeforeGeneration, AfterGeneration);

internal sealed record RuntimeV4ExpertRestResponse(
    RuntimeV4ExpertRestContext Context,
    string StateId,
    ulong Generation,
    RuntimeV4ExpertRestOperation Operation,
    RuntimeV4ExpertRestActionReference? Action,
    string Status,
    RuntimeV4ExpertGameplayObservation? Observation,
    RuntimeV4ExpertRestTransition? Transition,
    RuntimeV4ExpertRestEffectWitness? EffectWitness,
    string? ErrorCode);
