// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Screens.Settings;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static readonly Color RowSeparatorColor = new("2C434F", 0.5f);

    private static void AddRowSeparator(VBoxContainer parent)
    {
        var separator = new HSeparator { CustomMinimumSize = new Vector2(0, 1) };
        separator.AddThemeConstantOverride("separation", 1);
        separator.AddThemeStyleboxOverride(
            "separator",
            new StyleBoxFlat { BgColor = RowSeparatorColor, ContentMarginTop = 0, ContentMarginBottom = 0 });
        parent.AddChild(separator);
    }

    private static void RebuildFocusTargets(
        NSettingsPanel panel,
        VBoxContainer content,
        Control? preReadyFocusSentinel)
    {
        try
        {
            List<Control> focusables = new();
            CollectFocusableControls(content, focusables);

            if (preReadyFocusSentinel != null
                && GodotObject.IsInstanceValid(preReadyFocusSentinel)
                && preReadyFocusSentinel.GetParent() == content
                && focusables.Count > 0)
            {
                content.RemoveChild(preReadyFocusSentinel);
                preReadyFocusSentinel.QueueFree();
            }

            for (int i = 0; i < focusables.Count; i++)
            {
                Control control = focusables[i];
                control.FocusNeighborLeft = control.GetPath();
                control.FocusNeighborRight = control.GetPath();
                control.FocusNeighborTop = (i > 0 ? focusables[i - 1] : control).GetPath();
                control.FocusNeighborBottom = (i < focusables.Count - 1 ? focusables[i + 1] : control).GetPath();
            }

            FieldInfo? firstControlField = typeof(NSettingsPanel).GetField("_firstControl", PrivateInstance);
            firstControlField?.SetValue(panel, focusables.FirstOrDefault());
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile settings focus setup failed: {exception.GetType().Name}");
        }
    }

    private static void CollectFocusableControls(Control parent, List<Control> focusables)
    {
        foreach (Control child in parent.GetChildren().OfType<Control>())
        {
            if (!child.Visible) continue;
            if (child.FocusMode == Control.FocusModeEnum.All) focusables.Add(child);
            CollectFocusableControls(child, focusables);
        }
    }
}
