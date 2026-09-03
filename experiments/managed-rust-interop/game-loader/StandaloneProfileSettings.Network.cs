// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private sealed class BindAddressOption
    {
        internal BindAddressOption(string label, string value)
        {
            Label = label;
            Value = value;
        }

        internal string Label { get; }
        internal string Value { get; }
    }

    private static void PopulateNetworkSettings(VBoxContainer content)
    {
        EnsureSelectionLoaded();

        var section = new Label
        {
            Text = "Runtime connection",
            HorizontalAlignment = HorizontalAlignment.Left,
            CustomMinimumSize = new Vector2(0, 32)
        };
        section.AddThemeColorOverride("font_color", DimText);
        section.AddThemeFontSizeOverride("font_size", 20);
        ApplyGameFont(section);
        content.AddChild(section);

        var runtimeRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        runtimeRow.AddThemeConstantOverride("separation", 20);
        runtimeRow.AddChild(CreateRowLabel("Runtime API"));
        var runtimeToggle = new CheckButton
        {
            Text = RuntimeEnabled ? "On" : "Off",
            ButtonPressed = RuntimeEnabled,
            CustomMinimumSize = new Vector2(120, 0),
            FocusMode = Control.FocusModeEnum.All
        };
        runtimeToggle.Toggled += enabled => runtimeToggle.Text = enabled ? "On" : "Off";
        runtimeRow.AddChild(runtimeToggle);
        content.AddChild(runtimeRow);

        List<BindAddressOption> addressOptions = BuildBindAddressOptions();
        int selectedAddress = addressOptions.FindIndex(option =>
            string.Equals(option.Value, RuntimeBindAddress, StringComparison.OrdinalIgnoreCase));
        if (selectedAddress < 0)
        {
            addressOptions.Add(new BindAddressOption($"Saved address ({RuntimeBindAddress})", RuntimeBindAddress));
            selectedAddress = addressOptions.Count - 1;
        }

        var addressRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        addressRow.AddThemeConstantOverride("separation", 20);
        addressRow.AddChild(CreateRowLabel("Bind address"));
        var addressDropdown = new OptionButton
        {
            CustomMinimumSize = new Vector2(360, 0),
            FocusMode = Control.FocusModeEnum.All
        };
        foreach (BindAddressOption option in addressOptions)
        {
            addressDropdown.AddItem(option.Label);
        }

        addressDropdown.Selected = selectedAddress;
        addressRow.AddChild(addressDropdown);
        content.AddChild(addressRow);

        var portRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        portRow.AddThemeConstantOverride("separation", 20);
        portRow.AddChild(CreateRowLabel("Network port"));
        var portInput = new LineEdit
        {
            Text = RuntimePort.ToString(CultureInfo.InvariantCulture),
            PlaceholderText = "15526",
            MaxLength = 5,
            CustomMinimumSize = new Vector2(180, 0),
            FocusMode = Control.FocusModeEnum.All
        };
        portInput.AddThemeFontSizeOverride("font_size", 17);
        portRow.AddChild(portInput);
        content.AddChild(portRow);

        content.AddChild(CreateDescriptionLabel(
            $"Authentication: {ModEntry.RuntimeAuthenticationStatus}. The token remains outside the settings UI."));
        Label listenerStatus = CreateDescriptionLabel($"Listener: {ModEntry.RuntimeListenerStatus}.");
        content.AddChild(listenerStatus);

        Label status = CreateDescriptionLabel(
            "Network changes apply immediately and are saved for future launches. Select Apply after editing the values.");
        content.AddChild(status);

        addressDropdown.ItemSelected += index =>
        {
            BindAddressOption option = addressOptions[(int)index];
            status.Text = $"Unsaved bind address: {option.Label}. Select Apply to apply it now.";
        };
        portInput.TextChanged += _ =>
        {
            status.Text = "Unsaved network port. Select Apply to apply it now.";
        };

        runtimeToggle.Toggled += _ => status.Text = "Unsaved Runtime API setting. Select Apply to apply it now.";

        var actionRow = new HBoxContainer { CustomMinimumSize = new Vector2(0, 45) };
        actionRow.AddChild(new Control { SizeFlagsHorizontal = Control.SizeFlags.ExpandFill });
        var applyButton = new Button
        {
            Text = "Apply now",
            CustomMinimumSize = new Vector2(170, 34),
            FocusMode = Control.FocusModeEnum.All
        };
        applyButton.AddThemeFontSizeOverride("font_size", 17);
        applyButton.Pressed += () =>
        {
            BindAddressOption option = addressOptions[addressDropdown.Selected];
            if (TrySaveRuntimeSettings(runtimeToggle.ButtonPressed, option.Value, portInput.Text, out string error))
            {
                status.Text = ModEntry.ApplyRuntimeSettings();
                listenerStatus.Text = $"Listener: {ModEntry.RuntimeListenerStatus}.";
            }
            else
            {
                status.Text = error;
            }
        };
        actionRow.AddChild(applyButton);

        var resetButton = new Button
        {
            Text = "Reset",
            CustomMinimumSize = new Vector2(120, 34),
            FocusMode = Control.FocusModeEnum.All
        };
        resetButton.AddThemeFontSizeOverride("font_size", 17);
        resetButton.Pressed += () =>
        {
            if (!ResetRuntimeSettings(out string error))
            {
                status.Text = $"Could not reset network settings: {error}";
                return;
            }

            runtimeToggle.ButtonPressed = RuntimeEnabled;
            runtimeToggle.Text = RuntimeEnabled ? "On" : "Off";
            addressDropdown.Selected = addressOptions.FindIndex(option =>
                string.Equals(option.Value, RuntimeBindAddress, StringComparison.OrdinalIgnoreCase));
            portInput.Text = RuntimePort.ToString(CultureInfo.InvariantCulture);
            status.Text = ModEntry.ApplyRuntimeSettings();
            listenerStatus.Text = $"Listener: {ModEntry.RuntimeListenerStatus}.";
        };
        actionRow.AddChild(resetButton);
        content.AddChild(actionRow);

        AddRowSeparator(content);
    }

    private static List<BindAddressOption> BuildBindAddressOptions()
    {
        var options = new List<BindAddressOption>();
        var seenValues = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        void Add(string label, string value)
        {
            if (IsValidRuntimeBindAddress(value) && seenValues.Add(value))
            {
                options.Add(new BindAddressOption(label, value));
            }
        }

        Add("Localhost (127.0.0.1)", "127.0.0.1");
        Add("All network interfaces (0.0.0.0)", "0.0.0.0");

        try
        {
            string hostname = Dns.GetHostName();
            Add($"This computer ({hostname})", hostname);
            foreach (IPAddress address in Dns.GetHostEntry(hostname).AddressList)
            {
                if (address.AddressFamily == AddressFamily.InterNetwork)
                {
                    Add($"IPv4 ({address})", address.ToString());
                }
            }
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} could not enumerate local bind addresses: {exception.GetType().Name}");
        }

        return options;
    }
}
