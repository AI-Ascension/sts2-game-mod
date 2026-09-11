// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Bounded visible status data for the additive expert-state profile.</summary>
internal sealed record RuntimeV4ExpertGameplayStatus(
    string StatusId,
    string Name,
    int? Amount);

internal sealed record RuntimeV4ExpertGameplayRelic(
    string RelicId,
    string Name);

internal sealed record RuntimeV4ExpertGameplayPotion(
    string PotionId,
    string Name,
    byte? Slot,
    bool? Usable,
    string TargetMode);

/// <summary>Card metadata that is ordinary UI data when the host exposes it.</summary>
internal sealed record RuntimeV4ExpertGameplayCard(
    string CardId,
    string Name,
    byte? Cost,
    bool Upgraded,
    string? Type,
    string? Rarity,
    string? Target,
    string? Description);

internal sealed record RuntimeV4ExpertGameplayPlayer(
    ushort Hp,
    ushort MaxHp,
    ushort? Block,
    byte Energy,
    uint Gold)
{
    internal IReadOnlyList<RuntimeV4ExpertGameplayCard>? Hand { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayCard>? Deck { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayCard>? Discard { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayCard>? Exhaust { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayStatus>? Powers { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayStatus>? Statuses { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayRelic>? Relics { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayPotion>? Potions { get; init; }
    internal byte? PotionSlots { get; init; }
    internal byte? MaxPotionSlots { get; init; }
}

internal sealed record RuntimeV4ExpertGameplayRun(
    string? CharacterId,
    byte? Act,
    string? Location);

internal sealed record RuntimeV4ExpertGameplayIntent(
    string Kind,
    ushort? Damage,
    byte? Hits,
    IReadOnlyList<string>? TargetIds);

internal sealed record RuntimeV4ExpertGameplayEnemy(
    string EnemyId,
    string Name,
    ushort Hp,
    ushort MaxHp,
    ushort? Block,
    IReadOnlyList<RuntimeV4ExpertGameplayStatus>? Powers,
    IReadOnlyList<RuntimeV4ExpertGameplayStatus>? Statuses,
    RuntimeV4ExpertGameplayIntent Intent);

internal sealed record RuntimeV4ExpertGameplayMapNode(
    string NodeId,
    byte Act,
    byte Row,
    byte Col,
    string Kind,
    bool Reachable);

internal sealed record RuntimeV4ExpertGameplayMapEdge(
    string From,
    string To);

internal sealed record RuntimeV4ExpertGameplayChoice(
    string ChoiceId,
    string Label,
    string Kind,
    IReadOnlyList<string>? Domain);

internal sealed record RuntimeV4ExpertGameplayShopItem(
    string ItemId,
    string Name,
    string Kind,
    uint Price);

internal sealed record RuntimeV4ExpertGameplayState(string Kind)
{
    internal IReadOnlyList<string>? Characters { get; init; }
    internal string? CurrentNodeId { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayMapNode>? Nodes { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayMapEdge>? Edges { get; init; }
    internal IReadOnlyList<string>? Options { get; init; }
    internal ushort TurnIndex { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayEnemy>? Enemies { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayChoice>? Choices { get; init; }
    internal IReadOnlyList<RuntimeV4ExpertGameplayShopItem>? Items { get; init; }
    internal string? Reason { get; init; }
    internal string? Code { get; init; }
}

internal sealed record RuntimeV4ExpertGameplayAction(
    string ActionId,
    string Kind,
    string? Value,
    string? TargetId,
    string? SelectionId);

/// <summary>
/// Additive player-visible observation. Null means unavailable on the current native surface;
/// an empty collection means that the surface was observed and empty.
/// </summary>
internal sealed record RuntimeV4ExpertGameplayObservation(
    string StateId,
    ulong Generation,
    string? VisibleSeed,
    RuntimeV4ExpertGameplayRun Run,
    RuntimeV4ExpertGameplayPlayer Player,
    RuntimeV4ExpertGameplayState State,
    IReadOnlyList<RuntimeV4ExpertGameplayAction> LegalActions);
