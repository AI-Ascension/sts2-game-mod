// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeContentCatalogManifestSourceHelpers
{
    internal static object? CanonicalDynamicVars(object? dynamicVars)
    {
        if (dynamicVars is null)
            return null;
        PropertyInfo? keysProperty = dynamicVars.GetType().GetProperty("Keys");
        PropertyInfo? itemProperty = dynamicVars.GetType().GetProperty("Item");
        if (keysProperty?.GetValue(dynamicVars) is not IEnumerable keys || itemProperty is null)
            throw new InvalidOperationException("dynamic variable set shape changed");
        var values = new SortedDictionary<string, object?>(StringComparer.Ordinal);
        foreach (object? keyValue in keys)
        {
            if (keyValue is not string key
                || key.Length == 0
                || key.Length > 256
                || !ContentManifestWireContract.ValidIdentity(key))
            {
                throw new InvalidOperationException("dynamic variable key is malformed");
            }
            object? variable = itemProperty.GetValue(dynamicVars, new object[] { key });
            values[key] = CanonicalDynamicVar(variable);
            if (values.Count > MaxCollectionItems)
                throw new InvalidOperationException("dynamic variable set exceeds source bounds");
        }
        return values;
    }

    internal static object? ReadOptionalValue(object value, string name)
    {
        PropertyInfo? property = value.GetType().GetProperty(
            name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        return property?.GetValue(value);
    }

    private static Dictionary<string, object?>? CanonicalDynamicVar(object? variable)
    {
        if (variable is null)
            return null;
        return new Dictionary<string, object?>
        {
            ["type"] = variable.GetType().FullName,
            ["name"] = ReadProperty(variable, "Name"),
            ["base_value"] = ReadProperty(variable, "BaseValue"),
            ["int_value"] = ReadProperty(variable, "IntValue")
        };
    }

    private static void AddProperties(
        Dictionary<string, object?> target,
        AbstractModel model,
        IEnumerable<string> names)
    {
        foreach (string name in names)
        {
            object? value = ReadProperty(model, name);
            target[ToSemanticName(name)] = name is "CanonicalVars" or "DynamicVars"
                ? CanonicalDynamicVars(value)
                : CanonicalValue(value);
        }
    }

    private static string ToSemanticName(string name) =>
        char.ToLowerInvariant(name[0]) + name[1..];

    private static object? CanonicalValue(object? value, int depth = 0)
    {
        if (value is null)
            return null;
        if (depth > 3)
            throw new InvalidOperationException("model semantic nesting exceeds source bounds");
        switch (value)
        {
            case string:
            case bool:
            case byte:
            case sbyte:
            case short:
            case ushort:
            case int:
            case uint:
            case long:
            case ulong:
            case float:
            case double:
            case decimal:
                return value;
            case Enum:
                return value.ToString();
            case AbstractModel model:
                return new Dictionary<string, object?>
                {
                    ["category"] = model.Id.Category,
                    ["entry"] = model.Id.Entry
                };
        }
        Type type = value.GetType();
        if (type.Name == "ModelId")
        {
            return new Dictionary<string, object?>
            {
                ["category"] = ReadProperty(value, "Category"),
                ["entry"] = ReadProperty(value, "Entry")
            };
        }
        if (type.Name == "DynamicVarSet")
            return CanonicalDynamicVars(value);
        if (type.Name == "LocString")
        {
            return new Dictionary<string, object?>
            {
                ["loc_table"] = ReadProperty(value, "LocTable"),
                ["loc_entry_key"] = ReadProperty(value, "LocEntryKey")
            };
        }
        if (value is IEnumerable values)
        {
            var result = new List<object?>();
            foreach (object? item in values)
            {
                object? canonical = CanonicalValue(item, depth + 1);
                if (item is not null && canonical is null)
                    throw new InvalidOperationException("collection item shape is not canonical");
                result.Add(canonical);
                if (result.Count > MaxCollectionItems)
                    throw new InvalidOperationException("model collection exceeds source bounds");
            }
            return result.ToArray();
        }
        throw new InvalidOperationException(
            $"unsupported semantic value type: {value.GetType().FullName}");
    }

    private static object? LocalizedValue(object? value)
    {
        if (value is null)
            return null;
        MethodInfo? rawMethod = value.GetType().GetMethod(
            "GetRawText", BindingFlags.Public | BindingFlags.Instance,
            binder: null, Type.EmptyTypes, modifiers: null);
        return rawMethod?.Invoke(value, null) ?? CanonicalValue(value);
    }

    private static object? ReadProperty(object value, string name)
    {
        PropertyInfo property = value.GetType().GetProperty(
                name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)
            ?? throw new InvalidOperationException(
                $"host semantic property unavailable: {value.GetType().Name}.{name}");
        object? result = property.GetValue(value);
        if (result is string text
            && System.Text.Encoding.UTF8.GetByteCount(text) > MaxValueBytes)
        {
            throw new InvalidOperationException("host semantic value exceeds source bounds");
        }
        return result;
    }
}
