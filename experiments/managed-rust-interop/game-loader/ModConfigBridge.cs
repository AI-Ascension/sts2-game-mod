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
    private const string FrameworkAssemblyName = "ModConfig";
    private const string ApiName = "ModConfig.ModConfigApi";
    private const string EntryName = "ModConfig.ConfigEntry";
    private const string TypeName = "ModConfig.ConfigType";

    private static Type? _apiType;
    private static Type? _entryType;
    private static Type? _configType;
    private static MethodInfo? _register;
    private static MethodInfo? _getValue;
    private static MethodInfo? _setValue;
    private static Action? _ready;
    private static Action? _frame;
    private static Action? _applyUnlock;
    private static bool _requested;
    private static bool _readyCalled;
    private static bool _available;
    private static bool _registered;
    private static bool _persistenceDiagnosticLogged;

    internal static bool IsAvailable => _available && _registered;

    internal static void ConfigureCallbacks(Action? applyUnlock) => _applyUnlock = applyUnlock;

    internal static void DeferredRegister(Action settingsReady)
    {
        if (_requested) return;
        _requested = true;
        _ready = settingsReady;
        try
        {
            if (Engine.GetMainLoop() is not SceneTree tree) { FailOpen("SceneTree unavailable"); return; }

            Action callback = () => CompleteRegistration(tree); _frame = callback; tree.ProcessFrame += callback;
        }
        catch (Exception exception) { FailOpen(exception.GetType().Name); }
    }

    internal static bool GetBool(string key, bool fallback)
    {
        if (!KnownKey(key) || !IsAvailable || _getValue == null) return fallback;
        try
        {
            object? result = _getValue.MakeGenericMethod(typeof(bool)).Invoke(null, new object[] { ModId, key });
            return result is bool value ? value : fallback;
        }
        catch { return fallback; }
    }

    internal static bool SetBool(string key, bool value)
    {
        if (!KnownKey(key)) return false;
        if (!IsAvailable || _setValue == null)
        {
            ReportPersistenceFailure("API unavailable");
            return false;
        }

        try
        {
            _setValue.Invoke(null, new object[] { ModId, key, value });
            return true;
        }
        catch (Exception exception) { ReportPersistenceFailure(exception.GetType().Name); return false; }
    }

    private static void CompleteRegistration(SceneTree tree)
    {
        if (_frame is not null) { tree.ProcessFrame -= _frame; _frame = null; }

        try { DetectApi(); if (_available) RegisterSettings(); else FailOpen("API unavailable"); }
        catch (Exception exception) { FailOpen(exception.GetType().Name); }
        finally { InvokeReady(); }
    }

    private static void DetectApi()
    {
        _available = false;
        _registered = false;
        _register = null;
        _getValue = null;
        _setValue = null;
        Assembly? framework = FindFrameworkAssembly();
        if (framework == null) return;
        _apiType = FindPublicType(framework, ApiName);
        _entryType = FindPublicType(framework, EntryName);
        _configType = FindPublicType(framework, TypeName);
        if (_apiType == null || _entryType == null || _configType == null) return;
        _register = FindRegister(_apiType, _entryType);
        _getValue = FindGetValue(_apiType);
        _setValue = FindSetValue(_apiType);
        _available = _register != null && _getValue != null && _setValue != null;
    }

    private static Assembly? FindFrameworkAssembly()
    {
        Assembly? match = null;
        foreach (Assembly assembly in AppDomain.CurrentDomain.GetAssemblies())
        {
            try { if (!string.Equals(assembly.GetName().Name, FrameworkAssemblyName, StringComparison.Ordinal)) continue;
                if (match != null) return null; match = assembly; }
            catch { }
        }
        return match;
    }

    private static Type? FindPublicType(Assembly assembly, string fullName)
    {
        try { Type? type = assembly.GetType(fullName, false, false); return type is { IsPublic: true } ? type : null; }
        catch { return null; }
    }

    private static MethodInfo? FindRegister(Type api, Type entry)
    {
        MethodInfo? fallback = null;
        Type entries = entry.MakeArrayType();
        foreach (MethodInfo method in api.GetMethods(BindingFlags.Public | BindingFlags.Static))
        {
            if (method.Name != "Register") continue;
            ParameterInfo[] p = method.GetParameters();
            if (p.Length == 4 && p[0].ParameterType == typeof(string) && p[1].ParameterType == typeof(string)
                && p[2].ParameterType == typeof(Dictionary<string, string>) && p[3].ParameterType == entries)
            {
                return method;
            }
            else if (p.Length == 3 && p[0].ParameterType == typeof(string) && p[1].ParameterType == typeof(string)
                && p[2].ParameterType == entries)
            {
                fallback = method;
            }
        }
        return fallback;
    }

    private static MethodInfo? FindGetValue(Type api)
    {
        foreach (MethodInfo method in api.GetMethods(BindingFlags.Public | BindingFlags.Static))
        {
            Type[] genericArguments = method.GetGenericArguments();
            ParameterInfo[] parameters = method.GetParameters();
            if (method.Name == "GetValue" && method.IsGenericMethodDefinition && genericArguments.Length == 1
                && method.ReturnType == genericArguments[0] && parameters.Length == 2
                && parameters[0].ParameterType == typeof(string) && parameters[1].ParameterType == typeof(string))
            {
                return method;
            }
        }
        return null;
    }

    private static MethodInfo? FindSetValue(Type api)
    {
        foreach (MethodInfo method in api.GetMethods(BindingFlags.Public | BindingFlags.Static))
        {
            ParameterInfo[] parameters = method.GetParameters();
            if (method.Name == "SetValue" && !method.IsGenericMethod && method.ReturnType == typeof(void)
                && parameters.Length == 3 && parameters[0].ParameterType == typeof(string)
                && parameters[1].ParameterType == typeof(string) && parameters[2].ParameterType == typeof(object))
            {
                return method;
            }
        }
        return null;
    }

    private static void RegisterSettings()
    {
        if (_register == null || _entryType == null || _configType == null)
        { FailOpen("required API member unavailable"); return; }

        List<object> entries = new() { Header("Diagnostics"),
            Toggle(ShowDebugOverlayKey, "Show debug overlay on launch",
                "Diagnostic output only; does not change gameplay."),
            Header("Profile actions"),
            Toggle(UnlockOnNextLaunchKey, "Unlock all profile content on next launch",
                "One-shot action that changes profile progress on the next launch.") };
        if (TryCreateButton(out object? button)) entries.Add(button!);

        Array typedEntries = Array.CreateInstance(_entryType, entries.Count);
        for (int i = 0; i < entries.Count; i++) typedEntries.SetValue(entries[i], i);
        Dictionary<string, string> names = new() { ["en"] = "AI-Ascension STS2 POC" };
        object[] arguments = _register.GetParameters().Length == 4
            ? new object[] { ModId, names["en"], names, typedEntries } : new object[] { ModId, names["en"], typedEntries };
        _register.Invoke(null, arguments);
        _registered = true;
    }

    private static object Header(string label) => Entry(e => { Set(e, "Label", label); Set(e, "Type", EnumValue("Header")); });

    private static object Toggle(string key, string label, string description) => Entry(e =>
    { Set(e, "Key", key); Set(e, "Label", label); Set(e, "Type", EnumValue("Toggle"));
      Set(e, "DefaultValue", false); Set(e, "Description", description); });

    private static bool SupportsButton()
    {
        try { return _configType != null && _entryType != null && _applyUnlock != null
            && Enum.IsDefined(_configType, "Button") && AcceptsWritableValue("ButtonText", typeof(string))
            && AcceptsWritableValue("OnChanged", typeof(Action<object>)) && AcceptsWritableValue("Description", typeof(string)); }
        catch { return false; }
    }

    private static bool AcceptsWritableValue(string name, Type valueType)
    {
        PropertyInfo? property = _entryType?.GetProperty(name);
        return property is not null && property.CanWrite && property.GetSetMethod() is not null
            && property.PropertyType.IsAssignableFrom(valueType);
    }

    private static bool TryCreateButton(out object? button)
    {
        button = null;
        try
        {
            if (!SupportsButton()) return false;
            button = Entry(e => { Set(e, "Key", ApplyKey); Set(e, "Label", "Apply full profile unlock now");
                Set(e, "Type", EnumValue("Button")); Set(e, "ButtonText", "Apply");
                Set(e, "Description", "WARNING: this changes profile progress.");
                Set(e, "OnChanged", new Action<object>(_ => InvokeApply())); });
            return true;
        }
        catch { return false; }
    }

    private static object Entry(Action<object> configure)
    {
        if (_entryType == null) throw new InvalidOperationException("ConfigEntry unavailable");
        object entry = Activator.CreateInstance(_entryType) ?? throw new InvalidOperationException("ConfigEntry creation failed");
        configure(entry); return entry;
    }

    private static void Set(object entry, string name, object value)
    {
        PropertyInfo? property = entry.GetType().GetProperty(name);
        if (property is null || !property.CanWrite) throw new InvalidOperationException($"ConfigEntry member unavailable: {name}");
        property.SetValue(entry, value);
    }

    private static object EnumValue(string name)
    {
        if (_configType == null || !Enum.IsDefined(_configType, name)) throw new InvalidOperationException($"ConfigType value unavailable: {name}");
        return Enum.Parse(_configType, name, false);
    }

    private static bool KnownKey(string key) => string.Equals(key, ShowDebugOverlayKey, StringComparison.Ordinal)
        || string.Equals(key, UnlockOnNextLaunchKey, StringComparison.Ordinal);

    private static void InvokeApply() { try { _applyUnlock?.Invoke(); }
        catch (Exception exception) { GD.PrintErr($"{LogPrefix} settings action failed: {exception.GetType().Name}"); } }

    private static void ReportPersistenceFailure(string reason)
    { if (_persistenceDiagnosticLogged) return; _persistenceDiagnosticLogged = true;
      GD.PrintErr($"{LogPrefix} ModConfig setting persistence unavailable: {reason}"); }

    private static void InvokeReady()
    {
        if (_readyCalled) return;
        _readyCalled = true;
        Action? callback = _ready;
        _ready = null;
        try { callback?.Invoke(); }
        catch (Exception exception) { GD.PrintErr($"{LogPrefix} settings-ready callback failed: {exception.GetType().Name}"); }
    }

    private static void FailOpen(string reason)
    { _available = false; GD.PrintErr($"{LogPrefix} optional ModConfig unavailable: {reason}"); InvokeReady(); }
}
