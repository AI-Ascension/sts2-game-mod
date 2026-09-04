// SPDX-License-Identifier: MIT

using System;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static void PopulateRepeatSeedSettings(VBoxContainer content)
    {
        EnsureSelectionLoaded();

        var section = new Label
        {
            Text = "Repeat-seed behavior",
            HorizontalAlignment = HorizontalAlignment.Left,
            CustomMinimumSize = new Vector2(0, 32)
        };
        section.AddThemeColorOverride("font_color", DimText);
        section.AddThemeFontSizeOverride("font_size", 20);
        ApplyGameFont(section);
        content.AddChild(section);

        bool allowRepeatingSeeds = AllowRepeatingSeeds;
        var repeatSeedsRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        repeatSeedsRow.AddThemeConstantOverride("separation", 20);
        repeatSeedsRow.AddChild(CreateRowLabel("Allow repeating seeds"));
        var repeatSeedsToggle = new CheckButton
        {
            Text = allowRepeatingSeeds ? "On" : "Off",
            ButtonPressed = allowRepeatingSeeds,
            CustomMinimumSize = new Vector2(120, 0),
            FocusMode = Control.FocusModeEnum.All
        };
        Label repeatSeedStatus = CreateDescriptionLabel(
            "Enables the explicit one-shot replay/reset action for an active custom run.");
        repeatSeedsToggle.Toggled += enabled =>
        {
            bool previous = AllowRepeatingSeeds;
            if (!TrySaveAllowRepeatingSeeds(enabled, out string error))
            {
                repeatSeedsToggle.SetPressedNoSignal(previous);
                repeatSeedsToggle.Text = previous ? "On" : "Off";
                repeatSeedStatus.Text = BoundStatusText(
                    $"Could not save repeat-seed setting: {error}",
                    "Could not save repeat-seed setting.");
                return;
            }

            repeatSeedsToggle.Text = enabled ? "On" : "Off";
            repeatSeedStatus.Text = enabled
                ? "Saved. Replay/reset is limited to confirmed single-player custom runs."
                : "Saved. Replay/reset is disabled.";
        };
        repeatSeedsRow.AddChild(repeatSeedsToggle);
        content.AddChild(repeatSeedsRow);
        content.AddChild(repeatSeedStatus);

        var replayActionRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        replayActionRow.AddThemeConstantOverride("separation", 20);
        replayActionRow.AddChild(CreateRowLabel("One-shot action"));
        bool replayResetAvailable = AllowRepeatingSeeds && IsReplaySeedResetAvailable;
        var replayResetButton = new Button
        {
            Text = "Replay / reset seed once",
            Disabled = !replayResetAvailable,
            CustomMinimumSize = new Vector2(240, 34),
            FocusMode = Control.FocusModeEnum.All
        };
        replayResetButton.AddThemeFontSizeOverride("font_size", 17);
        replayActionRow.AddChild(replayResetButton);
        content.AddChild(replayActionRow);

        repeatSeedsToggle.Toggled += _ =>
        {
            replayResetButton.Disabled = !AllowRepeatingSeeds || !IsReplaySeedResetAvailable;
        };

        Label replayResetStatus = CreateDescriptionLabel(
            !AllowRepeatingSeeds
                ? "Enable repeating seeds to enable the explicit replay/reset action."
                : replayResetAvailable
                    ? "Requires a confirmation dialog; the current run is replaced without adding history."
                    : "Replay/reset is unavailable until the mod replay bridge is ready.");
        content.AddChild(replayResetStatus);
        repeatSeedsToggle.Toggled += enabled =>
        {
            replayResetStatus.Text = !enabled
                ? "Enable repeating seeds to enable the explicit replay/reset action."
                : IsReplaySeedResetAvailable
                    ? "Requires a confirmation dialog; the current run is replaced without adding history."
                    : "Replay/reset is unavailable until the mod replay bridge is ready.";
        };
        replayResetButton.Pressed += () =>
        {
            if (!AllowRepeatingSeeds || !IsReplaySeedResetAvailable)
            {
                replayResetButton.Disabled = true;
                replayResetStatus.Text = !AllowRepeatingSeeds
                    ? "Enable repeating seeds before requesting a reset."
                    : "Replay/reset is unavailable; no action was taken.";
                return;
            }

            var confirmation = new ConfirmationDialog
            {
                Title = "Replay this seed?",
                DialogText =
                    "The active custom run will be cleaned up and restarted from its original seed. "
                    + "Its current progress will be discarded. If restart fails after cleanup, "
                    + "the old run cannot be restored by this action. No run-history entry is requested. Continue?",
                OkButtonText = "Replay / reset",
                CancelButtonText = "Cancel"
            };
            Node? root = content.GetTree().Root;
            if (root == null)
            {
                confirmation.QueueFree();
                replayResetStatus.Text = "Replay/reset is unavailable; no action was taken.";
                return;
            }

            root.AddChild(confirmation);
            confirmation.Confirmed += () =>
            {
                confirmation.QueueFree();
                TryRequestReplaySeedReset(out string status);
                replayResetStatus.Text = status;
            };
            confirmation.Canceled += confirmation.QueueFree;
            confirmation.PopupCentered();
        };
        AddRowSeparator(content);
    }

    private static string BoundStatusText(string text, string fallback)
    {
        const int maxLength = 180;
        string normalized = string.IsNullOrWhiteSpace(text) ? fallback : text.Trim();
        return normalized.Length <= maxLength
            ? normalized
            : normalized[..(maxLength - 3)] + "...";
    }
}
