// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Saves;
using Environment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class VideoSettings
{
    private static string SettingsPath => Path.Combine(OS.GetUserDataDir(), "ai_ascension_video.settings");

    internal static VideoPreferences? Load()
    {
        try
        {
            var file = new FileInfo(SettingsPath);
            return file.Exists && file.Length <= 2048 ? VideoPreferences.Parse(File.ReadAllText(SettingsPath)) : null;
        }
        catch (IOException) { return null; }
        catch (UnauthorizedAccessException) { return null; }
    }

    internal static VideoPreferences Current()
    {
        Vector2I size = DisplayServer.WindowGetSize();
        string mode = DisplayServer.WindowGetMode() switch
        {
            DisplayServer.WindowMode.Fullscreen or DisplayServer.WindowMode.ExclusiveFullscreen => "fullscreen",
            DisplayServer.WindowMode.Maximized => "maximized",
            _ => DisplayServer.WindowGetFlag(DisplayServer.WindowFlags.Borderless) ? "borderless" : "windowed"
        };
        return new(DisplayServer.WindowGetCurrentScreen(), size.X, size.Y, mode);
    }

    internal static void ConfigureHost(VideoPreferences value)
    {
        if (!value.IsValid) throw new ArgumentException("Invalid video settings");
        int screen = value.ResolveDisplay(DisplayServer.GetScreenCount(), DisplayServer.GetPrimaryScreen());
        var settings = SaveManager.Instance.SettingsSave;
        settings.TargetDisplay = screen;
        settings.WindowSize = new Vector2I(value.Width, value.Height);
        settings.Fullscreen = value.Mode == "fullscreen";
        settings.WindowPosition = DisplayServer.ScreenGetPosition(screen);
    }

    private static void ApplyWindow(VideoPreferences value)
    {
        ConfigureHost(value);
        var settings = SaveManager.Instance.SettingsSave;
        DisplayServer.WindowSetMode(DisplayServer.WindowMode.Windowed);
        DisplayServer.WindowSetCurrentScreen(settings.TargetDisplay);
        DisplayServer.WindowSetFlag(DisplayServer.WindowFlags.Borderless, value.Mode == "borderless");
        DisplayServer.WindowSetSize(settings.WindowSize);
        DisplayServer.WindowSetPosition(settings.WindowPosition);
        DisplayServer.WindowSetMode(value.Mode switch
        {
            "fullscreen" => DisplayServer.WindowMode.Fullscreen,
            "maximized" => DisplayServer.WindowMode.Maximized,
            _ => DisplayServer.WindowMode.Windowed
        });
    }

    internal static async Task ApplyAsync(VideoPreferences value)
    {
        var tree = (SceneTree)Engine.GetMainLoop();
        if (DisplayServer.WindowGetMode() != DisplayServer.WindowMode.Windowed)
        {
            DisplayServer.WindowSetMode(DisplayServer.WindowMode.Windowed);
            // Let the desktop finish leaving fullscreen/maximized before requesting another mode.
            for (int frame = 0; frame < 12; frame++) await tree.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
        }
        ApplyWindow(value);
        for (int frame = 0; frame < 12; frame++) await tree.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
    }

    internal static async Task<string> ApplyAndSaveAsync(VideoPreferences value)
    {
        if (!value.IsValid || value.Display >= DisplayServer.GetScreenCount())
        {
            return "Select an available display and valid resolution.";
        }
        VideoPreferences before = Current();
        try
        {
            await ApplyAsync(value);
            VideoPreferences actual = Current();
            if (actual.Mode != value.Mode || actual.Display != value.ResolveDisplay(
                DisplayServer.GetScreenCount(), DisplayServer.GetPrimaryScreen()))
                throw new InvalidOperationException("The window manager did not apply the selected mode or display");
            VideoPreferences saved = value.Mode is "windowed" or "borderless"
                ? value with { Width = actual.Width, Height = actual.Height } : value;
            if (!saved.IsValid) throw new InvalidOperationException("The resulting window size is unsupported");
            SettingsPersistence.WriteAllLines(SettingsPath, new[] { JsonSerializer.Serialize(saved) });
            return saved == value ? "Video settings applied and saved."
                : $"Desktop adjusted the window to {saved.Width} × {saved.Height}. Actual size saved.";
        }
        catch (Exception error)
        {
            string message = "Could not apply or save video settings. Previous settings requested again.";
            try { await ApplyAsync(before); }
            catch (Exception restoreError)
            {
                message = "Could not save or restore video settings. Select another display mode.";
                GD.PrintErr($"[AI-ASCENSION VIDEO] restore failed: {restoreError.GetType().Name}");
            }
            GD.PrintErr($"[AI-ASCENSION VIDEO] apply failed: {error.GetType().Name}");
            return message;
        }
    }

    internal static void Initialize()
    {
        if (Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT") == "1"
            || Engine.GetMainLoop() is not SceneTree tree) return;
        int frames = 0;
        Action? restore = null;
        restore = async () =>
        {
            if (++frames < 300) return;
            tree.ProcessFrame -= restore;
            if (Load() is { } value)
            {
                try { await ApplyAsync(value); }
                catch (Exception error) { GD.PrintErr($"[AI-ASCENSION VIDEO] restore failed: {error.GetType().Name}"); }
            }
        };
        tree.ProcessFrame += restore;
    }
}
