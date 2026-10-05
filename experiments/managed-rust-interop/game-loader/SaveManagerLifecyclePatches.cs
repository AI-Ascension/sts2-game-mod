// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using HarmonyLib;
using MegaCrit.Sts2.Core.Saves;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class SaveManagerLifecyclePatches
{
    private static readonly object Gate = new();
    private static readonly ProgressReadinessPatchRegistration Registration = new();
    private static Harmony? _harmony;
    private static bool _attempted;
    private static bool _installed;
    private static ProgressReadinessState State => ProgressReadinessRuntime.State;

    internal static bool TryInstall()
    {
        lock (Gate)
        {
            if (_attempted)
            {
                return _installed;
            }

            _attempted = true;
            try
            {
                Assembly hostAssembly = typeof(SaveManager).Assembly;
                Assembly harmonyAssembly = typeof(Harmony).Assembly;
                if (!ProgressReadinessHostPin.IsSupported(hostAssembly, harmonyAssembly))
                {
                    State.Disable();
                    return false;
                }

                _harmony = new Harmony(ProgressReadinessPatchRegistration.OwnerId);
                _installed = Registration.TryInstall(State, new HarmonyPatchRegistry(_harmony));
                return _installed;
            }
            catch (Exception)
            {
                State.Disable();
                return false;
            }
        }
    }

    internal static void DisableReadiness()
    {
        try
        {
            State.Disable();
        }
        catch (Exception)
        {
        }
    }

    internal static bool TryCaptureOwnedSnapshot(out ProgressReadinessSnapshot snapshot)
    {
        return ProgressReadinessCallbacks.TryCaptureOwnedSnapshot(
            State,
            Environment.CurrentManagedThreadId,
            instance => instance is SaveManager manager
                ? ReadHostIdentity(manager)
                : throw new InvalidOperationException("Captured host identity has an unexpected type."),
            out snapshot);
    }

    private static void BeforeProfileInitialization() => RunInvalidation(State.BeforeProfileInitialization);

    private static void BeforeProfileSwitch() => RunInvalidation(State.BeforeProfileSwitch);

    private static void BeforeProfileDeletion() => RunInvalidation(State.BeforeProfileDeletion);

    private static void BeforeProgressReplacement() => RunInvalidation(State.BeforeProgressReplacement);

    private static void RunInvalidation(Action invalidate)
    {
        try
        {
            invalidate();
        }
        catch (Exception)
        {
            DisableReadiness();
        }
    }

    private static void BeginProgressLoad(
        SaveManager __instance,
        out ProgressReadinessLoadAttempt __state)
    {
        __state = default;
        try
        {
            __state = ProgressReadinessCallbacks.BeginProgressLoad(
                State,
                __instance,
                Environment.CurrentManagedThreadId,
                () => new ProgressReadinessHostIdentity(
                    __instance.IsProfileInitialized,
                    __instance.CurrentProfileId,
                    null));
        }
        catch (Exception)
        {
            ProgressReadinessRuntime.DisableSafely(State);
        }
    }

    private static void CompleteProgressLoad(
        SaveManager __instance,
        ReadSaveResult<SerializableProgress> __result,
        ProgressReadinessLoadAttempt __state)
    {
        try
        {
            ProgressReadinessCallbacks.CompleteProgressLoad(
                State,
                __state,
                __instance,
                Environment.CurrentManagedThreadId,
                () => (__result.Success, (int)__result.Status, __result.SaveData != null),
                () => ReadHostIdentity(__instance));
        }
        catch (Exception)
        {
            ProgressReadinessCallbacks.AbortProgressLoad(State, __state);
        }
    }

    private static Exception? PreserveHostException(
        Exception? __exception,
        ProgressReadinessLoadAttempt __state)
    {
        try
        {
            return ProgressReadinessCallbacks.FinalizeProgressLoad(State, __exception, __state);
        }
        catch (Exception)
        {
            ProgressReadinessRuntime.DisableSafely(State);
            return __exception;
        }
    }

    private static ProgressReadinessHostIdentity ReadHostIdentity(SaveManager manager) =>
        new(manager.IsProfileInitialized, manager.CurrentProfileId, manager.Progress);

    private static MethodInfo RequiredHostMethod(string name, params Type[] parameterTypes) =>
        typeof(SaveManager).GetMethod(
            name,
            BindingFlags.Instance | BindingFlags.Public,
            binder: null,
            types: parameterTypes,
            modifiers: null)
        ?? throw new MissingMethodException(typeof(SaveManager).FullName, name);

    private static MethodInfo RequiredPatchMethod(string name) =>
        typeof(SaveManagerLifecyclePatches).GetMethod(
            name,
            BindingFlags.Static | BindingFlags.NonPublic)
        ?? throw new MissingMethodException(typeof(SaveManagerLifecyclePatches).FullName, name);

    private sealed class HarmonyPatchRegistry : IProgressReadinessPatchRegistry
    {
        private readonly Harmony _harmony;

        internal HarmonyPatchRegistry(Harmony harmony)
        {
            _harmony = harmony;
        }

        public bool InstallAndVerifyOwner(string target, string ownerId)
        {
            MethodInfo original;
            MethodInfo? expectedPrefix = null;
            MethodInfo? expectedPostfix = null;
            MethodInfo? expectedFinalizer = null;
            bool needsPostfix = false;
            bool needsFinalizer = false;
            switch (target)
            {
                case "SaveManager.InitProfileId":
                    original = RequiredHostMethod(nameof(SaveManager.InitProfileId), typeof(int?));
                    expectedPrefix = RequiredPatchMethod(nameof(BeforeProfileInitialization));
                    _harmony.Patch(original, prefix: PatchMethod(expectedPrefix));
                    break;
                case "SaveManager.SwitchProfileId":
                    original = RequiredHostMethod(nameof(SaveManager.SwitchProfileId), typeof(int));
                    expectedPrefix = RequiredPatchMethod(nameof(BeforeProfileSwitch));
                    _harmony.Patch(original, prefix: PatchMethod(expectedPrefix));
                    break;
                case "SaveManager.DeleteProfile":
                    original = RequiredHostMethod(nameof(SaveManager.DeleteProfile), typeof(int));
                    expectedPrefix = RequiredPatchMethod(nameof(BeforeProfileDeletion));
                    _harmony.Patch(original, prefix: PatchMethod(expectedPrefix));
                    break;
                case "SaveManager.InitProgressData":
                    original = RequiredHostMethod(nameof(SaveManager.InitProgressData));
                    needsPostfix = true;
                    needsFinalizer = true;
                    expectedPrefix = RequiredPatchMethod(nameof(BeginProgressLoad));
                    expectedPostfix = RequiredPatchMethod(nameof(CompleteProgressLoad));
                    expectedFinalizer = RequiredPatchMethod(nameof(PreserveHostException));
                    _harmony.Patch(
                        original,
                        prefix: PatchMethod(expectedPrefix),
                        postfix: PatchMethod(expectedPostfix),
                        finalizer: PatchMethod(expectedFinalizer));
                    break;
                case "SaveManager.Progress.set":
                    MethodInfo setter = typeof(SaveManager).GetProperty(
                        nameof(SaveManager.Progress), BindingFlags.Instance | BindingFlags.Public)?.SetMethod
                        ?? throw new MissingMethodException(typeof(SaveManager).FullName, "set_Progress");
                    original = setter;
                    expectedPrefix = RequiredPatchMethod(nameof(BeforeProgressReplacement));
                    _harmony.Patch(original, prefix: PatchMethod(expectedPrefix));
                    break;
                default:
                    return false;
            }

            Patches? info = Harmony.GetPatchInfo(original);
            return info != null
                && HasExpectedMethod(info.Prefixes, ownerId, expectedPrefix)
                && HasExpectedMethod(info.Postfixes, ownerId, needsPostfix ? expectedPostfix : null)
                && HasExpectedMethod(info.Finalizers, ownerId, needsFinalizer ? expectedFinalizer : null);
        }

        private static HarmonyMethod PatchMethod(MethodInfo method) => new(method);

        private static bool HasExpectedMethod(
            IEnumerable<Patch> patches,
            string ownerId,
            MethodInfo? expectedMethod)
        {
            ProgressReadinessPatchBinding[] bindings = patches
                .Select(patch => new ProgressReadinessPatchBinding(patch.owner, patch.PatchMethod))
                .ToArray();
            return expectedMethod == null
                ? !bindings.Any(binding => string.Equals(binding.OwnerId, ownerId, StringComparison.Ordinal))
                : ProgressReadinessPatchVerifier.HasExactlyOneOwnedMethod(bindings, ownerId, expectedMethod);
        }
    }
}
