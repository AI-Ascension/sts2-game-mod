// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial record NativeVoteEncoding
{
    private static bool TryPlayerChoice(string value,
        out NativePlayerChoiceEncoding? encoding)
    {
        encoding = null;
        if (value == "skip")
        {
            encoding = NativePlayerChoiceEncoding.SingleIndex(null);
            return true;
        }

        if (TryIndex(value, "index:", out int index))
        {
            encoding = NativePlayerChoiceEncoding.SingleIndex(index);
            return true;
        }

        const string indexesPrefix = "indexes:";
        if (value.StartsWith(indexesPrefix, StringComparison.Ordinal))
        {
            string payload = value[indexesPrefix.Length..];
            string[] fields = payload.Split(',', StringSplitOptions.None);
            if (fields.Length is 0 or > 256)
                return false;
            var indexes = new List<int>(fields.Length);
            foreach (string field in fields)
            {
                if (!int.TryParse(field, NumberStyles.None,
                        CultureInfo.InvariantCulture, out int parsed)
                    || parsed < 0
                    || parsed > 4096)
                    return false;
                indexes.Add(parsed);
            }
            encoding = NativePlayerChoiceEncoding.MultipleIndexes(indexes);
            return true;
        }

        const string playerPrefix = "player:";
        if (value.StartsWith(playerPrefix, StringComparison.Ordinal)
            && ulong.TryParse(value[playerPrefix.Length..], NumberStyles.None,
                CultureInfo.InvariantCulture, out ulong playerId)
            && playerId != 0)
        {
            encoding = NativePlayerChoiceEncoding.Player(playerId);
            return true;
        }

        return false;
    }
}
