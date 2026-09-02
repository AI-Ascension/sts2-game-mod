// SPDX-License-Identifier: MIT

using System;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private const string ProfileArchiveFilter = "*.sts2profile ; STS2 profile export";
    private static bool _profileTransferDialogOpen;

    private static void AddProfileTransferControls(VBoxContainer content, Label status)
    {
        content.AddChild(CreateDescriptionLabel(
            "Export or import the selected profile's saved progress. Close active runs before transferring."));
        AddProfileTransferRow(content, "Export profile", "Export", profileId => OpenProfileExportDialog(profileId, status));
        AddProfileTransferRow(content, "Import profile", "Import", profileId => OpenProfileImportDialog(profileId, status));
        content.AddChild(CreateDescriptionLabel(
            "Imports replace saved progress in the selected profile and require a restart to load."));
        AddRowSeparator(content);
    }

    private static void AddProfileTransferRow(
        VBoxContainer content,
        string labelText,
        string buttonText,
        Action<int> action)
    {
        var row = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        row.AddThemeConstantOverride("separation", 20);
        row.AddChild(CreateRowLabel(labelText));

        var button = new Button
        {
            Text = buttonText,
            CustomMinimumSize = new Vector2(140, 34),
            FocusMode = Control.FocusModeEnum.All
        };
        button.AddThemeFontSizeOverride("font_size", 17);
        button.Pressed += () => action(ClampProfileId(_selectedProfileId));
        row.AddChild(button);
        content.AddChild(row);
    }

    private static void OpenProfileExportDialog(int profileId, Label status)
    {
        if (_profileTransferDialogOpen) return;
        if (!CanTransferProfile(out string error)
            || !TryGetProfileDirectory(profileId, requireExistingProgress: true, out _, out error))
        {
            SetProfileStatus(status, error);
            return;
        }

        var dialog = CreateProfileFileDialog(FileDialog.FileModeEnum.SaveFile);
        dialog.Title = $"Export Profile {profileId}";
        dialog.CurrentFile = $"Profile-{profileId}{ProfileArchiveExtension}";
        _profileTransferDialogOpen = true;
        dialog.FileSelected += path =>
        {
            dialog.QueueFree();
            SetProfileStatus(status, ExportProfile(profileId, path));
            _profileTransferDialogOpen = false;
        };
        dialog.Canceled += () =>
        {
            dialog.QueueFree();
            _profileTransferDialogOpen = false;
        };
        if (!TryShowProfileDialog(dialog))
        {
            _profileTransferDialogOpen = false;
            dialog.QueueFree();
            SetProfileStatus(status, "Profile export is unavailable because the settings window is not ready.");
        }
    }

    private static void OpenProfileImportDialog(int profileId, Label status)
    {
        if (_profileTransferDialogOpen) return;
        if (!CanTransferProfile(out string error))
        {
            SetProfileStatus(status, error);
            return;
        }

        var dialog = CreateProfileFileDialog(FileDialog.FileModeEnum.OpenFile);
        dialog.Title = $"Import into Profile {profileId}";
        _profileTransferDialogOpen = true;
        dialog.FileSelected += path =>
        {
            dialog.QueueFree();
            ShowImportConfirmation(profileId, path, status);
        };
        dialog.Canceled += () =>
        {
            dialog.QueueFree();
            _profileTransferDialogOpen = false;
        };
        if (!TryShowProfileDialog(dialog))
        {
            _profileTransferDialogOpen = false;
            dialog.QueueFree();
            SetProfileStatus(status, "Profile import is unavailable because the settings window is not ready.");
        }
    }

    private static FileDialog CreateProfileFileDialog(FileDialog.FileModeEnum mode)
    {
        var dialog = new FileDialog
        {
            FileMode = mode,
            Access = FileDialog.AccessEnum.Filesystem,
            Filters = new[] { ProfileArchiveFilter },
            CurrentDir = OS.GetUserDataDir()
        };
        return dialog;
    }

    private static bool TryShowProfileDialog(Window dialog)
    {
        if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
        {
            return false;
        }

        tree.Root.AddChild(dialog);
        dialog.PopupCenteredRatio(0.75f);
        return true;
    }

    private static void ShowImportConfirmation(int profileId, string archivePath, Label status)
    {
        var confirmation = new ConfirmationDialog
        {
            Title = $"Replace Profile {profileId}?",
            DialogText =
                $"Importing will replace all saved progress in Profile {profileId}. "
                + "The game must be restarted afterward. Continue?"
        };
        confirmation.Confirmed += () =>
        {
            confirmation.QueueFree();
            SetProfileStatus(status, ImportProfile(profileId, archivePath));
            _profileTransferDialogOpen = false;
        };
        confirmation.Canceled += () =>
        {
            confirmation.QueueFree();
            _profileTransferDialogOpen = false;
        };
        if (!TryShowProfileDialog(confirmation))
        {
            _profileTransferDialogOpen = false;
            confirmation.QueueFree();
            SetProfileStatus(status, "Profile import was canceled because the settings window is not ready.");
        }
    }

    private static void SetProfileStatus(Label status, string message)
    {
        if (GodotObject.IsInstanceValid(status))
        {
            status.Text = message;
        }
    }
}
