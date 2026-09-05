// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using Godot;
using MegaCrit.Sts2.Core.Modding;

namespace AiAscension.Sts2GameMod.Runtime;

[ModInitializer(nameof(Initialize))]
public static partial class ModEntry
{
    private const uint ExpectedAbiVersion = 1;
    private const int ExpectedCheckedAddStatus = 0;
    private const int ExpectedCheckedAddResult = 42;
    private const string DebugArgument = "--debug";
    private const string LogPrefix = "[AI-ASCENSION STS2 GAME MOD]";
    private const string StatusNodeName = "AIAscensionSTS2GameModStatus";
    private const string WorkshopAppIdVariable = "STS2_WORKSHOP_APP_ID";
    private const string WorkshopItemIdVariable = "STS2_WORKSHOP_ITEM_ID";
    private const string WorkshopGameVersionVariable = "STS2_WORKSHOP_GAME_VERSION";
    private const string WorkshopPlatformVariable = "STS2_WORKSHOP_PLATFORM";
    private static readonly object Gate = new();
    private static nint _nativeLibrary;
    private static Action? _statusOverlayCallback;
    private static bool _statusOverlayQueued;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate uint AbiVersion();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int CheckedAdd(int left, int right, out int output);

    public static void Initialize()
    {
        lock (Gate)
        {
            if (_nativeLibrary != 0)
            {
                GD.Print($"{LogPrefix} already initialized");
                return;
            }

            nint candidate = 0;

            try
            {
                string assemblyPath = Assembly.GetExecutingAssembly().Location;
                string directory = Path.GetDirectoryName(assemblyPath)
                    ?? throw new InvalidOperationException("The addon has no assembly directory.");
                ValidateWorkshopPackageIfPresent(directory);
                string nativePath = Path.Combine(directory, NativeLibraryFileName());
                candidate = NativeLibrary.Load(nativePath);

                nint export = NativeLibrary.GetExport(candidate, "sts2_game_mod_interop_abi_version");
                AbiVersion getVersion = Marshal.GetDelegateForFunctionPointer<AbiVersion>(export);
                uint version = getVersion();

                if (version != ExpectedAbiVersion)
                {
                    throw new InvalidOperationException($"ABI mismatch: expected {ExpectedAbiVersion}, found {version}.");
                }

                export = NativeLibrary.GetExport(candidate, "sts2_game_mod_interop_checked_add");
                CheckedAdd checkedAdd = Marshal.GetDelegateForFunctionPointer<CheckedAdd>(export);
                int status = checkedAdd(19, 23, out int sum);
                if (status != ExpectedCheckedAddStatus || sum != ExpectedCheckedAddResult)
                {
                    throw new InvalidOperationException(
                        $"native smoke call failed: status={status}, result={sum}");
                }

                _nativeLibrary = candidate;
                candidate = 0;
                GD.Print($"{LogPrefix} loaded managed entry point and Rust ABI; ABI={version}; 19+23={sum}");
                LiveCombatDemo.Initialize();
                InitializeRuntimeV3Gameplay();
                if (System.Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT") == "1")
                {
                    var source = new LiveCombatSource();
                    ConfigureRuntimeV3Gameplay(source, source);
                }
                StandaloneProfileSettings.Initialize();
                SeedReplayController.Initialize();
                StartRuntimeServer(_nativeLibrary);
                if (HasCommandLineArgument(DebugArgument))
                {
                    InstallStatusOverlay(version, sum);
                }

                AutoProfileUnlock.ScheduleLaunch();
            }
            catch (Exception exception)
            {
                if (candidate != 0)
                {
                    NativeLibrary.Free(candidate);
                }

                GD.PrintErr($"{LogPrefix} initialization failed: {exception.GetType().Name}: {exception.Message}");
            }
        }
    }

    private static bool HasCommandLineArgument(string expectedArgument)
    {
        foreach (string argument in System.Environment.GetCommandLineArgs())
        {
            if (string.Equals(argument, expectedArgument, StringComparison.Ordinal))
            {
                return true;
            }
        }

        return false;
    }

    private static void ValidateWorkshopPackageIfPresent(string directory)
    {
        string manifestPath = Path.Combine(directory, WorkshopPackageValidator.ManifestFileName);
        if (!File.Exists(manifestPath))
        {
            return;
        }

        uint appId = ReadPositiveUInt(WorkshopAppIdVariable);
        ulong itemId = ReadPositiveULong(WorkshopItemIdVariable);
        string gameVersion = ReadRequiredValue(WorkshopGameVersionVariable);
        string platform = ReadRequiredValue(WorkshopPlatformVariable);
        WorkshopPackageValidator.ValidateDirectory(directory, appId, itemId, gameVersion, platform);
        GD.Print($"{LogPrefix} first-party Workshop package validated: app={appId}; item={itemId}");
    }

    private static uint ReadPositiveUInt(string variable)
    {
        string value = ReadRequiredValue(variable);
        if (!uint.TryParse(value, out uint parsed) || parsed == 0)
        {
            throw new InvalidOperationException($"{variable} must be a positive decimal integer.");
        }

        return parsed;
    }

    private static ulong ReadPositiveULong(string variable)
    {
        string value = ReadRequiredValue(variable);
        if (!ulong.TryParse(value, out ulong parsed) || parsed == 0)
        {
            throw new InvalidOperationException($"{variable} must be a positive decimal integer.");
        }

        return parsed;
    }

    private static string ReadRequiredValue(string variable)
    {
        string? value = System.Environment.GetEnvironmentVariable(variable);
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new InvalidOperationException($"{variable} is required for a Workshop package.");
        }

        return value;
    }

    private static void InstallStatusOverlay(uint version, int sum)
    {
        if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
        {
            GD.PrintErr($"{LogPrefix} could not install visible status overlay: SceneTree root is unavailable");
            return;
        }

        if (tree.Root.GetNodeOrNull<CanvasLayer>(StatusNodeName) != null)
        {
            return;
        }

        if (_statusOverlayQueued)
        {
            return;
        }

        _statusOverlayQueued = true;
        _statusOverlayCallback = () =>
        {
            if (_statusOverlayCallback != null)
            {
                tree.ProcessFrame -= _statusOverlayCallback;
                _statusOverlayCallback = null;
            }

            _statusOverlayQueued = false;
            AddStatusOverlay(tree, version, sum, false);
        };
        tree.ProcessFrame += _statusOverlayCallback;
        GD.Print($"{LogPrefix} queued visible status overlay for the next safe frame");
    }

    private static void AddStatusOverlay(SceneTree tree, uint version, int sum, bool runtimeAction)
    {
        if (tree.Root == null || tree.Root.GetNodeOrNull<CanvasLayer>(StatusNodeName) != null)
        {
            return;
        }

        var layer = new CanvasLayer
        {
            Name = StatusNodeName,
            Layer = 1000
        };
        var background = new ColorRect
        {
            Name = "Background",
            Color = new Color(0.03f, 0.08f, 0.12f, 0.92f),
            Position = new Vector2(32, 32),
            Size = new Vector2(620, 108),
            MouseFilter = Control.MouseFilterEnum.Ignore
        };
        var label = new Label
        {
            Name = "Message",
            Text = runtimeAction
                ? $"AI-ASCENSION STS2\nLIVE RUNTIME | ABI {version} | effect witnessed"
                : $"AI-ASCENSION STS2 GAME MOD\nDEBUG | Rust ABI {version} | 19 + 23 = {sum}",
            Position = new Vector2(52, 48),
            Size = new Vector2(580, 76),
            MouseFilter = Control.MouseFilterEnum.Ignore
        };
        label.AddThemeFontSizeOverride("font_size", 22);
        label.AddThemeColorOverride("font_color", new Color(0.96f, 0.89f, 0.68f, 1.0f));
        label.AddThemeColorOverride("font_shadow_color", new Color(0, 0, 0, 0.85f));
        label.AddThemeConstantOverride("shadow_offset_x", 2);
        label.AddThemeConstantOverride("shadow_offset_y", 2);

        layer.AddChild(background);
        layer.AddChild(label);
        tree.Root.AddChild(layer);
        GD.Print($"{LogPrefix} visible status overlay installed: {StatusNodeName}");
    }

    private static string NativeLibraryFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return "AIAscensionSTS2GameModNative.dll";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return "libAIAscensionSTS2GameModNative.dylib";
        }

        return "libAIAscensionSTS2GameModNative.so";
    }
}
