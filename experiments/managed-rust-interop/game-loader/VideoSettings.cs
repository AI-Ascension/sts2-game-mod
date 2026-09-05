// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Text.Json;
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

    internal static void Apply(VideoPreferences value)
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

    internal static bool TryApplyAndSave(VideoPreferences value, out string message)
    {
        if (!value.IsValid || value.Display >= DisplayServer.GetScreenCount())
        {
            message = "Select an available display and valid resolution.";
            return false;
        }
        VideoPreferences before = Current();
        try
        {
            Apply(value);
            SettingsPersistence.WriteAllLines(SettingsPath, new[] { JsonSerializer.Serialize(value) });
            message = "Video settings applied and saved.";
            return true;
        }
        catch (Exception error)
        {
            message = "Could not save video settings. Previous display restored.";
            try { Apply(before); }
            catch (Exception restoreError)
            {
                message = "Could not save or restore video settings. Select another display mode.";
                GD.PrintErr($"[AI-ASCENSION VIDEO] restore failed: {restoreError.GetType().Name}");
            }
            GD.PrintErr($"[AI-ASCENSION VIDEO] apply failed: {error.GetType().Name}");
            return false;
        }
    }

    internal static void Initialize()
    {
        if (Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT") == "1"
            || Engine.GetMainLoop() is not SceneTree tree) return;
        int frames = 0;
        Action? restore = null;
        restore = () =>
        {
            if (++frames < 300) return;
            tree.ProcessFrame -= restore;
            if (Load() is { } value) Apply(value);
        };
        tree.ProcessFrame += restore;
    }
}
