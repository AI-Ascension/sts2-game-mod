// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Security.Cryptography;
using System.Text;
using Godot;
using MegaCrit.Sts2.Core.CardSelection;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Cards;
using MegaCrit.Sts2.Core.Nodes.Cards.Holders;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.GodotExtensions;
using MegaCrit.Sts2.Core.Nodes.Screens.CardSelection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private static string Fingerprint(
        NSimpleCardSelectScreen screen,
        CardModel[] cards,
        IEnumerable<int> visibleIndexes,
        IEnumerable<int> selectedIndexes)
    {
        var payload = new StringBuilder(screen.GetType().FullName);
        payload.Append('|').Append(cards.Length).Append('|');
        for (int index = 0; index < cards.Length; index++)
        {
            CardModel card = cards[index];
            payload.Append(index).Append(':').Append(card.Id).Append(':')
                .Append(RuntimeHelpers.GetHashCode(card)).Append(';');
        }
        payload.Append("visible:");
        foreach (int index in visibleIndexes.OrderBy(index => index))
            payload.Append(index).Append(',');
        payload.Append("|selected:");
        foreach (int index in selectedIndexes.OrderBy(index => index))
            payload.Append(index).Append(',');
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(payload.ToString())))
            .ToLowerInvariant();
    }
}
