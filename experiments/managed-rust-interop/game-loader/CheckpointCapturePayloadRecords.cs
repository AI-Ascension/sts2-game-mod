// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Owned managed records shaped exactly like <c>schemas/checkpoint-payload-v1.schema.json</c>
/// (ADR 0057). Every value is a copied primitive, string, or nested owned record; no record
/// holds a host object. Field names follow the schema property names in PascalCase.
/// </summary>
internal enum CheckpointPayloadCoverage
{
    Captured,
    Unknown,
    NotApplicable
}

internal static class CheckpointPayloadCoverageCodes
{
    internal static string Code(this CheckpointPayloadCoverage coverage) => coverage switch
    {
        CheckpointPayloadCoverage.Captured => "captured",
        CheckpointPayloadCoverage.Unknown => "unknown",
        CheckpointPayloadCoverage.NotApplicable => "not_applicable",
        _ => throw new ArgumentOutOfRangeException(nameof(coverage))
    };
}

/// <summary>One coverage family: a captured value, an explicit unknown, or not applicable.</summary>
internal sealed record CheckpointPayloadFamily<T>(CheckpointPayloadCoverage Coverage, T? Value)
    where T : class
{
    internal static CheckpointPayloadFamily<T> Captured(T value)
    {
        ArgumentNullException.ThrowIfNull(value);
        return new CheckpointPayloadFamily<T>(CheckpointPayloadCoverage.Captured, value);
    }

    internal static CheckpointPayloadFamily<T> Unknown() =>
        new(CheckpointPayloadCoverage.Unknown, null);

    internal static CheckpointPayloadFamily<T> NotApplicable() =>
        new(CheckpointPayloadCoverage.NotApplicable, null);
}

internal sealed record CheckpointPayloadRngStream(
    string StreamId,
    string Algorithm,
    ulong Cursor,
    IReadOnlyList<ulong> StateWords);

internal sealed record CheckpointPayloadSeedAndRng(
    string MasterSeed,
    string DerivationVersion,
    IReadOnlyList<CheckpointPayloadRngStream> Streams);

internal sealed record CheckpointPayloadRunConfiguration(
    string Character,
    long Ascension,
    string Mode,
    IReadOnlyList<string> ActSequence,
    IReadOnlyList<string> Modifiers);

internal sealed record CheckpointPayloadCampaignProgress(
    long ActIndex,
    long Floor,
    string CurrentNode,
    string RoomKind,
    IReadOnlyList<string> VisitedNodes);

internal sealed record CheckpointPayloadPlayerResources(
    long CurrentHp,
    long MaxHp,
    long Gold,
    IReadOnlyList<string> Keys);

internal sealed record CheckpointPayloadCardPile(string PileId, IReadOnlyList<string> Cards);

internal sealed record CheckpointPayloadDeckAndPiles(
    IReadOnlyList<string> Deck,
    IReadOnlyList<CheckpointPayloadCardPile> Piles);

internal sealed record CheckpointPayloadTemporaryValue(string Key, long Value);

internal sealed record CheckpointPayloadCardInstance(
    string InstanceId,
    string DefinitionId,
    long UpgradeLevel,
    IReadOnlyList<CheckpointPayloadTemporaryValue> TemporaryValues);

internal sealed record CheckpointPayloadCardInstances(
    IReadOnlyList<CheckpointPayloadCardInstance> Cards);

internal sealed record CheckpointPayloadRelic(string RelicId, long Counter);

internal sealed record CheckpointPayloadRelics(IReadOnlyList<CheckpointPayloadRelic> Relics);

internal sealed record CheckpointPayloadPotionSlot(long Index, string PotionId);

internal sealed record CheckpointPayloadPotions(
    long Capacity,
    IReadOnlyList<CheckpointPayloadPotionSlot> Slots);

internal sealed record CheckpointPayloadCombatTurn(long Turn, long Energy, long PlayerBlock);

internal sealed record CheckpointPayloadPower(string Owner, string PowerId, long Amount);

internal sealed record CheckpointPayloadPowers(IReadOnlyList<CheckpointPayloadPower> Powers);

internal sealed record CheckpointPayloadIntent(string IntentId, long Value);

internal sealed record CheckpointPayloadEnemy(
    string EnemyId,
    long CurrentHp,
    long MaxHp,
    long Block,
    IReadOnlyList<CheckpointPayloadIntent> Intents);

internal sealed record CheckpointPayloadEnemiesAndIntents(
    IReadOnlyList<CheckpointPayloadEnemy> Enemies);

/// <summary><c>Mode</c> is <c>controlled</c> or <c>absent</c>; the value is never carried.</summary>
internal sealed record CheckpointPayloadExternalInput(string InputId, string Mode);

internal sealed record CheckpointPayloadPendingEffects(
    IReadOnlyList<string> PendingEffects,
    IReadOnlyList<CheckpointPayloadExternalInput> ExternalInputs);

/// <summary>The twelve ADR 0037 families in schema order.</summary>
internal sealed record CheckpointPayloadFamilies(
    CheckpointPayloadFamily<CheckpointPayloadSeedAndRng> SeedAndRng,
    CheckpointPayloadFamily<CheckpointPayloadRunConfiguration> RunConfiguration,
    CheckpointPayloadFamily<CheckpointPayloadCampaignProgress> CampaignProgress,
    CheckpointPayloadFamily<CheckpointPayloadPlayerResources> PlayerResources,
    CheckpointPayloadFamily<CheckpointPayloadDeckAndPiles> DeckAndPiles,
    CheckpointPayloadFamily<CheckpointPayloadCardInstances> CardInstances,
    CheckpointPayloadFamily<CheckpointPayloadRelics> Relics,
    CheckpointPayloadFamily<CheckpointPayloadPotions> Potions,
    CheckpointPayloadFamily<CheckpointPayloadCombatTurn> CombatTurn,
    CheckpointPayloadFamily<CheckpointPayloadPowers> Powers,
    CheckpointPayloadFamily<CheckpointPayloadEnemiesAndIntents> EnemiesAndIntents,
    CheckpointPayloadFamily<CheckpointPayloadPendingEffects> PendingEffects);

/// <summary>One owned <c>ascension.checkpoint_payload.v1</c> record for a first-release boundary.</summary>
internal sealed record CheckpointPayloadRecord(
    CheckpointCaptureBoundary Boundary,
    CheckpointPayloadFamilies Families)
{
    internal const string Schema = "ascension.checkpoint_payload.v1";
    internal const string CanonicalProfile = "asc-jcs-state-v1";
}
