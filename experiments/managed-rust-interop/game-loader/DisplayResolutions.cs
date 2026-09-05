// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class DisplayResolutions
{
    internal static List<Vector2I> ForDisplay(int requested)
    {
        int screen = requested >= 0 && requested < DisplayServer.GetScreenCount()
            ? requested : DisplayServer.GetPrimaryScreen();
        Vector2I position = DisplayServer.ScreenGetPosition(screen);
        Vector2I desktop = DisplayServer.ScreenGetSize(screen);
        var choices = new HashSet<Vector2I> { desktop };
        try
        {
            foreach (var size in WindowsDisplayModes.Read(position.X, position.Y, desktop.X, desktop.Y))
                if (size.Width >= 640 && size.Height >= 360 && size.Width <= desktop.X && size.Height <= desktop.Y)
                    choices.Add(new Vector2I(size.Width, size.Height));
        }
        catch (Exception error) when (error is DllNotFoundException or EntryPointNotFoundException)
        {
            GD.PrintErr("[AI-ASCENSION VIDEO] display modes unavailable; using detected desktop size");
        }
        return choices.OrderBy(size => size.X).ThenBy(size => size.Y).ToList();
    }
}
