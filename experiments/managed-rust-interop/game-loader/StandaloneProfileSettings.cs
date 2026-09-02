// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Nodes.Screens.Settings;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Owns the mod's small profile-settings panel without requiring a settings framework mod.
/// The host does not expose a public tab-registration API, so the narrow tab seam is accessed
/// through the native settings manager's private tab dictionary.
/// </summary>
internal static partial class StandaloneProfileSettings
{
    private const string LogPrefix = "[AI-ASCENSION STS2 POC]";
    private const string TabName = "AiAscensionProfileSettings";
    private const string TabLabel = "AI-Ascension";
    private const string PanelName = "AiAscensionProfileSettingsPanel";
    private const int MinProfileId = 1;
    private const int MaxProfileId = 3;
    private static readonly Color DimText = new("8A7E5C");
    private static readonly Color TextColor = new(0.9f, 0.85f, 0.75f);
    private static bool _initialized;
    private static Font? _gameFont;

    internal static void Initialize()
    {
        if (_initialized) return;
        _initialized = true;
        EnsureSelectionLoaded();

        if (Engine.GetMainLoop() is not SceneTree tree)
        {
            GD.PrintErr($"{LogPrefix} could not install profile settings: SceneTree is unavailable");
            return;
        }

        tree.NodeAdded += OnNodeAdded;
    }

    private static Control? CreatePreReadyFocusSentinel(NSettingsPanel firstPanel)
    {
        try
        {
            if (firstPanel.DefaultFocusedControl is Control defaultFocused
                && GodotObject.IsInstanceValid(defaultFocused)
                && defaultFocused.Duplicate() is Control duplicate)
            {
                duplicate.FocusMode = Control.FocusModeEnum.All;
                duplicate.Visible = false;
                return duplicate;
            }
        }
        catch
        {
            // Fall through to searching for another native settings control.
        }

        try
        {
            if (firstPanel.Content is Control content)
            {
                Control? candidate = FindFirstGameSettingsControl(content);
                if (candidate != null && candidate.Duplicate() is Control duplicate)
                {
                    duplicate.FocusMode = Control.FocusModeEnum.All;
                    duplicate.Visible = false;
                    return duplicate;
                }
            }
        }
        catch
        {
            // A missing sentinel only disables controller focus fallback.
        }

        return null;
    }

    private static Control? FindFirstGameSettingsControl(Control parent)
    {
        foreach (Control child in parent.GetChildren().OfType<Control>())
        {
            if (child.GetType().Name is "NTickbox" or "NSettingsSlider" or "NDropdownPositioner" or "NPaginator")
            {
                return child;
            }

            Control? found = FindFirstGameSettingsControl(child);
            if (found != null) return found;
        }

        return null;
    }

    private static void CacheGameFont(NSettingsPanel panel)
    {
        if (_gameFont != null || panel.Content == null) return;
        try
        {
            foreach (Node child in panel.Content.GetChildren())
            {
                if (child is Label label)
                {
                    _gameFont = label.GetThemeFont("font");
                    if (_gameFont != null) return;
                }

                if (child is not Control container) continue;
                foreach (Node inner in container.GetChildren())
                {
                    if (inner is Label innerLabel)
                    {
                        _gameFont = innerLabel.GetThemeFont("font");
                        if (_gameFont != null) return;
                    }
                }
            }
        }
        catch
        {
            // The default Godot font is an acceptable fallback.
        }
    }

    private static void PopulateProfileSettings(VBoxContainer content)
    {
        EnsureSelectionLoaded();

        var section = new Label
        {
            Text = "Profile actions",
            HorizontalAlignment = HorizontalAlignment.Left,
            CustomMinimumSize = new Vector2(0, 32)
        };
        section.AddThemeColorOverride("font_color", DimText);
        section.AddThemeFontSizeOverride("font_size", 20);
        ApplyGameFont(section);
        content.AddChild(section);

        var profileRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        profileRow.AddThemeConstantOverride("separation", 20);
        var profileLabel = CreateRowLabel("Target profile");
        var profileDropdown = new OptionButton
        {
            CustomMinimumSize = new Vector2(220, 0),
            FocusMode = Control.FocusModeEnum.All
        };
        for (int profileId = MinProfileId; profileId <= MaxProfileId; profileId++)
        {
            profileDropdown.AddItem($"Profile {profileId}", profileId - MinProfileId);
        }

        profileDropdown.Selected = _selectedProfileId - MinProfileId;
        var status = CreateDescriptionLabel($"Profile {_selectedProfileId} is selected. Only the selected profile will be modified.");
        profileDropdown.ItemSelected += index =>
        {
            _selectedProfileId = ClampProfileId((int)index + MinProfileId);
            SaveSelection();
            status.Text = $"Profile {_selectedProfileId} is selected. Only the selected profile will be modified.";
        };
        profileRow.AddChild(profileLabel);
        profileRow.AddChild(profileDropdown);
        content.AddChild(profileRow);
        content.AddChild(status);
        AddRowSeparator(content);

        var applyRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        applyRow.AddThemeConstantOverride("separation", 20);
        var applyLabel = CreateRowLabel("Apply full profile unlock");
        var applyButton = new Button
        {
            Text = "Apply",
            CustomMinimumSize = new Vector2(140, 34),
            FocusMode = Control.FocusModeEnum.All
        };
        applyButton.AddThemeFontSizeOverride("font_size", 17);
        applyButton.Pressed += () =>
        {
            int profileId = ClampProfileId(_selectedProfileId);
            AutoProfileUnlock.ScheduleManualUnlock(profileId);
            status.Text = $"Unlock queued for Profile {profileId}.";
        };
        applyRow.AddChild(applyLabel);
        applyRow.AddChild(applyButton);
        content.AddChild(applyRow);
        content.AddChild(CreateDescriptionLabel("Changes progress only in the selected profile."));
    }

    private static Label CreateRowLabel(string text)
    {
        var label = new Label
        {
            Text = $"  {text}",
            SizeFlagsHorizontal = Control.SizeFlags.ExpandFill
        };
        label.AddThemeColorOverride("font_color", TextColor);
        label.AddThemeFontSizeOverride("font_size", 20);
        ApplyGameFont(label);
        return label;
    }

    private static Label CreateDescriptionLabel(string text)
    {
        var label = new Label
        {
            Text = $"      {text}",
            CustomMinimumSize = new Vector2(0, 20),
            AutowrapMode = TextServer.AutowrapMode.WordSmart
        };
        label.AddThemeColorOverride("font_color", DimText);
        label.AddThemeFontSizeOverride("font_size", 15);
        ApplyGameFont(label);
        return label;
    }

    private static void ApplyGameFont(Label label)
    {
        if (_gameFont != null) label.AddThemeFontOverride("font", _gameFont);
    }
}
