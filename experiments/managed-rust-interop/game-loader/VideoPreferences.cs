// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record VideoPreferences(int Display, int Width, int Height, string Mode)
{
    internal static readonly VideoPreferences Default = new(-1, 1280, 720, "windowed");

    internal bool IsValid => Display is >= -1 and <= 31 && Width is >= 640 and <= 16384
        && Height is >= 360 and <= 16384 && Mode is "windowed" or "fullscreen" or "borderless" or "maximized";

    internal int ResolveDisplay(int count, int primary) => Display >= 0 && Display < count ? Display : primary;

    internal static VideoPreferences? Parse(string text)
    {
        if (text.Length > 2048) return null;
        try
        {
            VideoPreferences? value = JsonSerializer.Deserialize<VideoPreferences>(text);
            return value?.IsValid == true ? value : null;
        }
        catch (JsonException) { return null; }
    }
}
