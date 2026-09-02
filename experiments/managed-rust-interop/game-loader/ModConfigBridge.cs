// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class ModConfigBridge
{
    internal const string ModId = "AIAscensionSTS2Poc";
    internal const string ShowDebugOverlayKey = "show_debug_overlay";
    internal const string UnlockOnNextLaunchKey = "unlock_all_on_next_launch";

    private const string ApplyKey = "apply_full_profile_unlock_now";
    private const string LogPrefix = "[AI-ASCENSION STS2 POC]";
    private const string ApiName = "ModConfig.ModConfigApi";
    private const string EntryName = "ModConfig.ConfigEntry";
    private const string TypeName = "ModConfig.ConfigType";

    private static Type? _apiType;
    private static Type? _entryType;
    private static Type? _configType;
    private static MethodInfo? _register;
    private static Action? _ready;
    private static Action? _frame;
    private static Action? _applyUnlock;
    private static bool _requested;
    private static bool _readyCalled;
    private static bool _available;
    private static bool _registered;

    internal static bool IsAvailable => _available && _registered;

    internal static void ConfigureCallbacks(Action? applyUnlock) => _applyUnlock = applyUnlock;

    internal static void DeferredRegister(Action settingsReady)
    {
        if (_requested) return;
        _requested = true;
        _ready = settingsReady;
        try
        {
            if (Engine.GetMainLoop() is not SceneTree tree)
            {
                FailOpen("SceneTree unavailable");
                return;
            }

            Action callback = () => CompleteRegistration(tree);
            _frame = callback;
            tree.ProcessFrame += callback;
        }
        catch (Exception exception)
        {
            FailOpen(exception.GetType().Name);
        }
    }

    internal static bool GetBool(string key, bool fallback)
    {
        if (!KnownKey(key) || !IsAvailable || _apiType == null) return fallback;
        try
        {
            MethodInfo? method = _apiType.GetMethod("GetValue", BindingFlags.Public | BindingFlags.Static);
            if (method == null || !method.IsGenericMethodDefinition) return fallback;
            object? result = method.MakeGenericMethod(typeof(bool))
                .Invoke(null, new object[] { ModId, key });
            return result is bool value ? value : fallback;
        }
        catch
        {
            return fallback;
        }
    }

    internal static void SetBool(string key, bool value)
    {
        if (!KnownKey(key) || !IsAvailable || _apiType == null) return;
        try
        {
            _apiType.GetMethod("SetValue", BindingFlags.Public | BindingFlags.Static)
                ?.Invoke(null, new object[] { ModId, key, value });
        }
        catch
        {
        }
    }

    private static void CompleteRegistration(SceneTree tree)
    {
        if (_frame != null)
        {
            tree.ProcessFrame -= _frame;
            _frame = null;
        }

        try
        {
            DetectApi();
            if (_available) RegisterSettings(); else FailOpen("API unavailable");
        }
        catch (Exception exception)
        {
            FailOpen(exception.GetType().Name);
        }
        finally
        {
            InvokeReady();
        }
    }

    private static void DetectApi()
    {
        _available = false;
        _apiType = FindPublicType(ApiName);
        _entryType = FindPublicType(EntryName);
        _configType = FindPublicType(TypeName);
        if (_apiType == null || _entryType == null || _configType == null) return;
        _register = FindRegister(_apiType, _entryType);
        _available = _register != null;
    }

    private static Type? FindPublicType(string fullName)
    {
        foreach (Assembly assembly in AppDomain.CurrentDomain.GetAssemblies())
        {
            try
            {
                Type? type = assembly.GetType(fullName, false, false);
                if (type != null && type.IsPublic) return type;
            }
            catch
            {
            }
        }
        return null;
    }

    private static MethodInfo? FindRegister(Type api, Type entry)
    {
        MethodInfo? localized = null;
        MethodInfo? fallback = null;
        Type entries = entry.MakeArrayType();
        foreach (MethodInfo method in api.GetMethods(BindingFlags.Public | BindingFlags.Static))
        {
            if (method.Name != "Register") continue;
            ParameterInfo[] p = method.GetParameters();
            if (p.Length == 4 && p[0].ParameterType == typeof(string)
                && p[1].ParameterType == typeof(string)
                && p[2].ParameterType == typeof(Dictionary<string, string>)
                && p[3].ParameterType == entries)
            {
                localized = method;
            }
            else if (p.Length == 3 && p[0].ParameterType == typeof(string)
                && p[1].ParameterType == typeof(string) && p[2].ParameterType == entries)
            {
                fallback = method;
            }
        }
        return localized ?? fallback;
    }

    private static void RegisterSettings()
    {
        if (_register == null || _entryType == null || _configType == null)
        {
            FailOpen("required API member unavailable");
            return;
        }

        List<object> entries = new()
        {
            Entry(e => { Set(e, "Label", "Diagnostics"); Set(e, "Type", EnumValue("Header")); }),
            Entry(e =>
            {
                Set(e, "Key", ShowDebugOverlayKey);
                Set(e, "Label", "Show debug overlay on launch");
                Set(e, "Type", EnumValue("Toggle"));
                Set(e, "DefaultValue", false);
                Set(e, "Description", "Diagnostic output only; does not change gameplay.");
            }),
            Entry(e => { Set(e, "Label", "Profile actions"); Set(e, "Type", EnumValue("Header")); }),
            Entry(e =>
            {
                Set(e, "Key", UnlockOnNextLaunchKey);
                Set(e, "Label", "Unlock all profile content on next launch");
                Set(e, "Type", EnumValue("Toggle"));
                Set(e, "DefaultValue", false);
                Set(e, "Description", "One-shot action that changes profile progress on the next launch.");
            })
        };
        if (SupportsButton())
        {
            entries.Add(Entry(e =>
            {
                Set(e, "Key", ApplyKey);
                Set(e, "Label", "Apply full profile unlock now");
                Set(e, "Type", EnumValue("Button"));
                Set(e, "ButtonText", "Apply");
                Set(e, "OnChanged", new Action<object>(_ => InvokeApply()));
            }));
        }

        Array typedEntries = Array.CreateInstance(_entryType, entries.Count);
        for (int i = 0; i < entries.Count; i++) typedEntries.SetValue(entries[i], i);
        Dictionary<string, string> names = new() { ["en"] = "AI-Ascension STS2 POC" };
        ParameterInfo[] parameters = _register.GetParameters();
        object[] arguments = parameters.Length == 4
            ? new object[] { ModId, names["en"], names, typedEntries }
            : new object[] { ModId, names["en"], typedEntries };
        _register.Invoke(null, arguments);
        _registered = true;
    }

    private static bool SupportsButton() => _configType != null && _entryType != null
        && Enum.IsDefined(_configType, "Button") && _applyUnlock != null
        && _entryType.GetProperty("ButtonText") != null && _entryType.GetProperty("OnChanged") != null;

    private static object Entry(Action<object> configure)
    {
        if (_entryType == null) throw new InvalidOperationException("ConfigEntry unavailable");
        object entry = Activator.CreateInstance(_entryType) ?? throw new InvalidOperationException("ConfigEntry creation failed");
        configure(entry); return entry;
    }

    private static void Set(object entry, string name, object value)
    {
        PropertyInfo? property = entry.GetType().GetProperty(name);
        if (property == null || !property.CanWrite) throw new InvalidOperationException($"ConfigEntry member unavailable: {name}");
        property.SetValue(entry, value);
    }

    private static object EnumValue(string name)
    {
        if (_configType == null || !Enum.IsDefined(_configType, name))
            throw new InvalidOperationException($"ConfigType value unavailable: {name}");
        return Enum.Parse(_configType, name, false);
    }

    private static bool KnownKey(string key)
    {
        return string.Equals(key, ShowDebugOverlayKey, StringComparison.Ordinal)
            || string.Equals(key, UnlockOnNextLaunchKey, StringComparison.Ordinal);
    }

    private static void InvokeApply()
    {
        try { _applyUnlock?.Invoke(); }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} settings action failed: {exception.GetType().Name}");
        }
    }

    private static void InvokeReady()
    {
        if (_readyCalled) return;
        _readyCalled = true;
        Action? callback = _ready;
        _ready = null;
        try { callback?.Invoke(); }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} settings-ready callback failed: {exception.GetType().Name}");
        }
    }

    private static void FailOpen(string reason)
    {
        _available = false;
        GD.PrintErr($"{LogPrefix} optional ModConfig unavailable: {reason}");
        InvokeReady();
    }
}
