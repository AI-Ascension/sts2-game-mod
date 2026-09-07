// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Reflection;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static RuntimeV4ExpertGameplayStatus[]? PublicStatuses(object? host, string propertyName)
    {
        object? raw = PublicValue(host, propertyName);
        if (raw is not IEnumerable values || raw is string) return null;
        var result = new List<RuntimeV4ExpertGameplayStatus>();
        try
        {
            foreach (object? value in values)
            {
                string? id = ReadNestedIdentity(value, "Id") ?? PublicText(value, "Name");
                string? name = PublicText(value, "Name") ?? id;
                if (id is null || name is null) return null;
                result.Add(new RuntimeV4ExpertGameplayStatus(id, name, PublicInt(value, "Amount")));
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static object? PublicValue(object? host, string propertyName) => host?.GetType()
        .GetProperty(propertyName, BindingFlags.Instance | BindingFlags.Public)?.GetValue(host);

    private static bool? PublicBool(object? host, string propertyName) => PublicValue(host, propertyName) switch
    {
        bool value => value, _ => null
    };

    private static ushort? PublicU16(object? host, string propertyName)
    {
        int? value = PublicInt(host, propertyName);
        return value is >= 0 and <= ushort.MaxValue ? (ushort)value.Value : null;
    }

    private static int? PublicInt(object? host, string propertyName) => PublicValue(host, propertyName) switch
    {
        int value => value, short value => value, byte value => value,
        long value when value is >= int.MinValue and <= int.MaxValue => (int)value, _ => null
    };

    private static string? PublicText(object? host, string propertyName) =>
        VisibleText(PublicValue(host, propertyName));

    private static string? VisibleText(object? value)
    {
        if (value is string text && RuntimeV3GameplayContract.IsText(text)) return text;
        if (value is Enum enumValue && RuntimeV3GameplayContract.IsIdentity(enumValue.ToString()))
            return enumValue.ToString();
        if (value is null) return null;
        foreach (string methodName in new[] { "GetFormattedText", "GetText" })
        {
            try
            {
                MethodInfo? method = value.GetType().GetMethod(
                    methodName, BindingFlags.Instance | BindingFlags.Public,
                    binder: null, Type.EmptyTypes, modifiers: null);
                if (method?.Invoke(value, null) is string formatted
                    && RuntimeV3GameplayContract.IsText(formatted)) return formatted;
            }
            catch (Exception)
            {
                // A missing or invalid localizer leaves the field unavailable.
            }
        }
        return null;
    }

    private static string? ReadNestedIdentity(object? host, params string[] properties)
    {
        object? current = host;
        foreach (string property in properties) current = PublicValue(current, property);
        return current switch
        {
            string value when RuntimeV3GameplayContract.IsIdentity(value) => value, _ => null
        };
    }
}
