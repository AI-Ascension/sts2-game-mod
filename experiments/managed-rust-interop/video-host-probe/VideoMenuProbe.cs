// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Opt-in disposable-host probe; excluded from normal addon builds.</summary>
internal static class VideoMenuProbe
{
    internal static async Task RunAsync(SceneTree tree)
    {
        try
        {
            var mainMenu = NGame.Instance?.MainMenu ?? throw new InvalidOperationException("main menu unavailable");
            mainMenu.OpenSettingsMenu();
            for (int i = 0; i < 30; i++) await tree.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
            var display = (OptionButton)tree.Root.FindChild("VideoDisplay", true, false);
            var resolution = (OptionButton)tree.Root.FindChild("VideoResolution", true, false);
            var mode = (OptionButton)tree.Root.FindChild("VideoMode", true, false);
            var apply = (Button)tree.Root.FindChild("VideoApply", true, false);
            for (int screen = 0; screen < DisplayServer.GetScreenCount(); screen++)
            {
                display.Select(screen + 1);
                display.EmitSignal(OptionButton.SignalName.ItemSelected, screen + 1);
                var sizes = DisplayResolutions.ForDisplay(screen);
                Check(resolution.ItemCount == sizes.Count && sizes.Count > 0, "display resolution refresh");
                GD.Print($"[VIDEO PROBE] display={screen}; modes={string.Join(',', sizes.Select(s => $"{s.X}x{s.Y}"))}");
            }
            display.Select(0);
            display.EmitSignal(OptionButton.SignalName.ItemSelected, 0);
            var primarySizes = DisplayResolutions.ForDisplay(-1);
            int selected = primarySizes.FindLastIndex(s => s.X <= 1024 && s.Y <= 768);
            if (selected < 0) selected = primarySizes.Count - 1;
            resolution.Select(selected);
            foreach (int modeIndex in new[] { 2, 0 })
            {
                mode.Select(modeIndex);
                mode.EmitSignal(OptionButton.SignalName.ItemSelected, modeIndex);
                apply.EmitSignal(Button.SignalName.Pressed);
                for (int i = 0; i < 30; i++) await tree.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
                var saved = VideoSettings.Load();
                var actual = VideoSettings.Current();
                Check(saved != null && saved.Display == -1 && actual.Mode == saved.Mode
                    && actual.Width == saved.Width && actual.Height == saved.Height
                    && actual.Display == DisplayServer.GetPrimaryScreen(), "apply and persist selected display/mode/size");
                GD.Print($"[VIDEO PROBE] saved={saved!.Mode}; actual={actual.Mode}; size={actual.Width}x{actual.Height}; display={actual.Display}");
            }
            Node tab = tree.Root.FindChild("AiAscensionProfileSettings", true, false);
            tab.GetParent().Call("SwitchTabTo", tab);
            for (int i = 0; i < 30; i++) await tree.ToSignal(tree, SceneTree.SignalName.ProcessFrame);
            Check(display.IsVisibleInTree() && apply.IsVisibleInTree(), "video menu visible");
            var scroll = (ScrollContainer)tree.Root.FindChild("AiAscensionSettingsScroll", true, false);
            Check(scroll.GetVScrollBar().MaxValue > scroll.GetVScrollBar().Page, "settings content scrolls");
            tree.Root.GetTexture().GetImage().SavePng("user://video-menu-probe.png");
            GD.Print("[VIDEO PROBE] PASS: native menu display catalogs, apply, persistence, and visibility");
        }
        catch (Exception error) { GD.PrintErr($"[VIDEO PROBE] FAIL: {error.GetType().Name}: {error.Message}"); }
    }

    private static void Check(bool value, string message)
    {
        if (!value) throw new InvalidOperationException(message);
    }
}
