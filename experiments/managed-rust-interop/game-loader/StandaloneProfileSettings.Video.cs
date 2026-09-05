// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static void PopulateVideoSettings(VBoxContainer content)
    {
        var title = CreateRowLabel("Video");
        title.AddThemeColorOverride("font_color", DimText);
        content.AddChild(title);
        VideoPreferences current = VideoSettings.Current();
        var display = VideoDropdown(content, "Display", "VideoDisplay");
        display.AddItem("Primary display", -1);
        for (int i = 0; i < DisplayServer.GetScreenCount(); i++)
        {
            Vector2I size = DisplayServer.ScreenGetSize(i);
            display.AddItem($"Display {i + 1} — {size.X} × {size.Y}" +
                (i == DisplayServer.GetPrimaryScreen() ? " (Primary)" : ""), i);
        }
        // OptionButton assigns negative IDs automatically, so the first item is mapped explicitly.
        display.Selected = current.Display + 1;
        var resolution = VideoDropdown(content, "Resolution", "VideoResolution");
        var sizes = new List<Vector2I>();
        Vector2I selectedSize = new(current.Width, current.Height);
        void RefreshResolutions()
        {
            if (resolution.Selected >= 0 && resolution.Selected < sizes.Count) selectedSize = sizes[resolution.Selected];
            sizes = DisplayResolutions.ForDisplay(display.Selected - 1);
            resolution.Clear();
            foreach (Vector2I size in sizes) resolution.AddItem($"{size.X} × {size.Y}");
            int selected = sizes.IndexOf(selectedSize);
            resolution.Selected = selected >= 0 ? selected : sizes.Count - 1;
        }
        RefreshResolutions();
        display.ItemSelected += _ => RefreshResolutions();
        var mode = VideoDropdown(content, "Window mode", "VideoMode");
        string[] modes = { "windowed", "fullscreen", "borderless", "maximized" };
        foreach (string label in new[] { "Windowed", "Fullscreen", "Borderless window", "Maximized" }) mode.AddItem(label);
        mode.Selected = Array.IndexOf(modes, current.Mode);
        resolution.Disabled = current.Mode is "fullscreen" or "maximized";
        mode.ItemSelected += index => resolution.Disabled = modes[(int)index] is "fullscreen" or "maximized";
        content.AddChild(CreateDescriptionLabel("Fullscreen and maximized use the display size. Resolution applies to windowed modes."));
        var apply = new Button { Name = "VideoApply", Text = "Apply video settings",
            CustomMinimumSize = new Vector2(220, 36), FocusMode = Control.FocusModeEnum.All };
        var status = CreateDescriptionLabel("Changes apply immediately and are saved for future launches.");
        status.Name = "VideoStatus";
        apply.Pressed += () =>
        {
            Vector2I size = sizes[resolution.Selected];
            if (!DisplayResolutions.ForDisplay(display.Selected - 1).Contains(size))
            {
                RefreshResolutions();
                status.Text = "Display modes changed. Select a resolution and apply again.";
                return;
            }
            var selected = new VideoPreferences(display.Selected - 1, size.X, size.Y, modes[mode.Selected]);
            VideoSettings.TryApplyAndSave(selected, out string message);
            status.Text = message;
        };
        content.AddChild(apply);
        content.AddChild(status);
        AddRowSeparator(content);
    }

    private static OptionButton VideoDropdown(VBoxContainer content, string label, string name)
    {
        var row = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        row.AddChild(CreateRowLabel(label));
        var dropdown = new OptionButton { Name = name, CustomMinimumSize = new Vector2(340, 36),
            FocusMode = Control.FocusModeEnum.All };
        row.AddChild(dropdown);
        content.AddChild(row);
        return dropdown;
    }
}
