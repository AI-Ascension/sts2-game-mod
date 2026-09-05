// SPDX-License-Identifier: MIT

using System;
using Godot;
using MegaCrit.Sts2.Core.Saves;
using Environment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class LiveCombatDisplay
{
    internal static void ConfigureSettings()
    {
        int screen = Number("DISPLAY", 0, 0, DisplayServer.GetScreenCount() - 1);
        int width = Number("WIDTH", 1280, 640, 16384);
        int height = Number("HEIGHT", 720, 360, 16384);
        var settings = SaveManager.Instance.SettingsSave;
        settings.TargetDisplay = screen;
        settings.WindowSize = new Vector2I(width, height);
        settings.Fullscreen = Mode() == "fullscreen";
        settings.WindowPosition = DisplayServer.ScreenGetPosition(screen);
    }

    internal static void Apply()
    {
        var settings = SaveManager.Instance.SettingsSave;
        string mode = Mode();
        DisplayServer.WindowSetMode(DisplayServer.WindowMode.Windowed);
        DisplayServer.WindowSetCurrentScreen(settings.TargetDisplay);
        DisplayServer.WindowSetSize(settings.WindowSize);
        DisplayServer.WindowSetPosition(settings.WindowPosition);
        DisplayServer.WindowSetFlag(DisplayServer.WindowFlags.Borderless, mode == "borderless");
        DisplayServer.WindowSetMode(mode switch
        {
            "fullscreen" => DisplayServer.WindowMode.Fullscreen,
            "maximized" => DisplayServer.WindowMode.Maximized,
            _ => DisplayServer.WindowMode.Windowed
        });
        GD.Print($"[AI-ASCENSION LIVE] display={DisplayServer.WindowGetCurrentScreen()}; " +
            $"size={DisplayServer.WindowGetSize()}; mode={DisplayServer.WindowGetMode()}");
    }

    private static string Mode()
    {
        string mode = Environment.GetEnvironmentVariable("STS2_LIVE_WINDOW_MODE") ?? "windowed";
        return mode is "windowed" or "fullscreen" or "borderless" or "maximized"
            ? mode : throw new InvalidOperationException("invalid launch window mode");
    }

    private static int Number(string key, int fallback, int min, int max)
    {
        string? text = Environment.GetEnvironmentVariable("STS2_LIVE_" + key);
        if (text == null) return fallback;
        return int.TryParse(text, out int value) && value >= min && value <= max ? value
            : throw new InvalidOperationException("invalid launch display setting: " + key);
    }
}
