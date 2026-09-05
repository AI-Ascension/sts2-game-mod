// SPDX-License-Identifier: MIT

using AiAscension.Sts2GameMod.Runtime;
using System.Text.Json;

internal static class VideoPreferencesTests
{
    internal static void Run()
    {
        foreach (string mode in new[] { "windowed", "fullscreen", "borderless", "maximized" })
        {
            var value = new VideoPreferences(-1, 1280, 720, mode);
            Check(VideoPreferences.Parse(JsonSerializer.Serialize(value)) == value, "Video settings round trip");
        }
        Check(new VideoPreferences(2, 1280, 720, "windowed").ResolveDisplay(2, 0) == 0,
            "Disconnected screen falls back to primary");
        Check(VideoPreferences.Default.ResolveDisplay(3, 2) == 2, "Primary is not assumed to be screen zero");
        foreach (string invalid in new[] { "{}", "not json", new string('x', 2049),
            "{\"Display\":0,\"Width\":-1,\"Height\":720,\"Mode\":\"windowed\"}",
            "{\"Display\":0,\"Width\":1280,\"Height\":720,\"Mode\":\"invalid\"}" })
            Check(VideoPreferences.Parse(invalid) == null, "Invalid saved video settings rejected");
        Console.WriteLine("Video preference persistence and disconnected-display checks passed");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
