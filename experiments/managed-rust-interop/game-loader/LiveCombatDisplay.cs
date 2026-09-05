// SPDX-License-Identifier: MIT

using System;
using Godot;
using MegaCrit.Sts2.Core.Saves;
using Environment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class LiveCombatDisplay
{
    private static VideoPreferences _selected = VideoPreferences.Default;
    internal static void ConfigureSettings()
    {
        VideoPreferences saved = VideoSettings.Load() ?? VideoPreferences.Default;
        int screen = Number("DISPLAY", saved.Display, -1, DisplayServer.GetScreenCount() - 1);
        int width = Number("WIDTH", saved.Width, 640, 16384);
        int height = Number("HEIGHT", saved.Height, 360, 16384);
        _selected = new(screen, width, height, Mode(saved.Mode));
        for (int index = 0; index < DisplayServer.GetScreenCount(); index++)
            GD.Print($"[AI-ASCENSION LIVE] available display={index}; " +
                $"position={DisplayServer.ScreenGetPosition(index)}; size={DisplayServer.ScreenGetSize(index)}; " +
                $"primary={index == DisplayServer.GetPrimaryScreen()}");
        VideoSettings.ConfigureHost(_selected);
    }

    internal static void Apply()
    {
        VideoSettings.Apply(_selected);
        GD.Print($"[AI-ASCENSION LIVE] display={DisplayServer.WindowGetCurrentScreen()}; " +
            $"size={DisplayServer.WindowGetSize()}; mode={DisplayServer.WindowGetMode()}; " +
            $"borderless={DisplayServer.WindowGetFlag(DisplayServer.WindowFlags.Borderless)}");
    }

    private static string Mode(string fallback)
    {
        string mode = Environment.GetEnvironmentVariable("STS2_LIVE_WINDOW_MODE") ?? fallback;
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
