// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV3GameplayCard(
    string CardId,
    string Name,
    byte Cost,
    bool Upgraded);

internal sealed record RuntimeV3GameplayEnemy(
    string EnemyId,
    string Name,
    ushort Hp,
    ushort MaxHp,
    RuntimeV3GameplayIntent Intent,
    ushort IntentDamage,
    byte IntentHits);

internal sealed record RuntimeV3GameplayShopItem(
    string ItemId,
    string Name,
    uint Price);

internal sealed record RuntimeV3GameplayPlayer(
    ushort Hp,
    ushort MaxHp,
    byte Energy,
    uint Gold,
    IReadOnlyList<RuntimeV3GameplayCard> Hand,
    IReadOnlyList<RuntimeV3GameplayCard> Deck,
    IReadOnlyList<RuntimeV3GameplayCard> Discard,
    IReadOnlyList<RuntimeV3GameplayCard> Exhaust);

/// <summary>
/// A managed observation assembled from ordinary player-visible signals. It intentionally has no
/// host object, save, executable, random-state, or unrevealed-outcome field.
/// </summary>
internal sealed record RuntimeV3GameplayObservation(
    string StateId,
    ulong Generation,
    string? VisibleSeed,
    RuntimeV3GameplayPlayer Player,
    RuntimeV3GameplayState State,
    IReadOnlyList<string> StateValues,
    IReadOnlyList<RuntimeV3GameplayEnemy> Enemies)
{
    internal ushort TurnIndex { get; init; }
    internal string? NodeId { get; init; }
    internal IReadOnlyList<RuntimeV3GameplayShopItem> ShopItems { get; init; } =
        Array.Empty<RuntimeV3GameplayShopItem>();
    internal bool IsActionable { get; init; }
    internal bool ModalBlocking { get; init; }
    internal bool InputEnabled { get; init; }

    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(StateId)
            || Generation > RuntimeV3GameplayContract.MaxGeneration
            || VisibleSeed is not null && !RuntimeV3GameplayContract.IsText(VisibleSeed)
            || Player.Hp > Player.MaxHp
            || NodeId is not null && !RuntimeV3GameplayContract.IsIdentity(NodeId)
            || !Enum.IsDefined(State)
            || State == RuntimeV3GameplayState.Unknown)
        {
            error = "observation identity, state, resource, or generation is invalid";
            return false;
        }

        return ValidateCards(out error)
            && ValidateStateValues(out error)
            && ValidateVisibleEntities(out error);
    }

    private bool ValidateCards(out string error)
    {
        foreach (IReadOnlyList<RuntimeV3GameplayCard> cards in new[]
        {
            Player.Hand,
            Player.Deck,
            Player.Discard,
            Player.Exhaust
        })
        {
            if (cards.Count > RuntimeV3GameplayContract.MaxEntities)
            {
                error = "observation card collection exceeds its bound";
                return false;
            }
            foreach (RuntimeV3GameplayCard card in cards)
            {
                if (!RuntimeV3GameplayContract.IsIdentity(card.CardId)
                    || !RuntimeV3GameplayContract.IsText(card.Name))
                {
                    error = "observation card projection is invalid";
                    return false;
                }
            }
        }

        error = string.Empty;
        return true;
    }

    private bool ValidateStateValues(out string error)
    {
        if (StateValues.Count > RuntimeV3GameplayContract.MaxEntities
            || Enemies.Count > RuntimeV3GameplayContract.MaxEntities
            || ShopItems.Count > RuntimeV3GameplayContract.MaxEntities)
        {
            error = "observation collection exceeds its bound";
            return false;
        }

        if (State == RuntimeV3GameplayState.Defeat
            && (StateValues.Count > 1
                || StateValues.Count == 1 && !RuntimeV3GameplayContract.IsText(StateValues[0])))
        {
            error = "observation defeat reason is invalid";
            return false;
        }

        if (State == RuntimeV3GameplayState.Recovery
            && (StateValues.Count != 1 || !RuntimeV3GameplayContract.IsIdentity(StateValues[0])))
        {
            error = "observation recovery code is invalid";
            return false;
        }

        foreach (string value in StateValues)
        {
            if (State != RuntimeV3GameplayState.Defeat
                && !RuntimeV3GameplayContract.IsIdentity(value))
            {
                error = "observation state value is invalid";
                return false;
            }
        }

        error = string.Empty;
        return true;
    }

    private bool ValidateVisibleEntities(out string error)
    {
        foreach (RuntimeV3GameplayEnemy enemy in Enemies)
        {
            if (!RuntimeV3GameplayContract.IsIdentity(enemy.EnemyId)
                || !RuntimeV3GameplayContract.IsText(enemy.Name)
                || enemy.Hp > enemy.MaxHp
                || !Enum.IsDefined(enemy.Intent)
                || enemy.Intent == RuntimeV3GameplayIntent.Attack && enemy.IntentHits == 0)
            {
                error = "observation enemy projection is invalid";
                return false;
            }
        }

        foreach (RuntimeV3GameplayShopItem item in ShopItems)
        {
            if (!RuntimeV3GameplayContract.IsIdentity(item.ItemId)
                || !RuntimeV3GameplayContract.IsText(item.Name))
            {
                error = "observation shop projection is invalid";
                return false;
            }
        }

        error = string.Empty;
        return true;
    }
}
