// SPDX-License-Identifier: MIT

using System;
using System.Globalization;
using System.IO;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private const string SettingsFileName = "ai_ascension_sts2_poc.settings";
    private const int DefaultProfileId = 1;
    private const int DefaultRuntimePort = 15526;
    private const string DefaultRuntimeBindAddress = "127.0.0.1";
    private const int MinRuntimePort = 1024;
    private const int MaxRuntimePort = ushort.MaxValue;
    private static bool _selectionLoaded;
    private static int _selectedProfileId = DefaultProfileId;
    private static bool _runtimeEnabled = true;
    private static int _runtimePort = DefaultRuntimePort;
    private static string _runtimeBindAddress = DefaultRuntimeBindAddress;

    internal static bool RuntimeEnabled
    {
        get
        {
            EnsureSelectionLoaded();
            return _runtimeEnabled;
        }
    }

    internal static int RuntimePort
    {
        get
        {
            EnsureSelectionLoaded();
            return _runtimePort;
        }
    }

    internal static string RuntimeBindAddress
    {
        get
        {
            EnsureSelectionLoaded();
            return _runtimeBindAddress;
        }
    }

    internal static bool IsValidRuntimeBindAddress(string value)
    {
        if (string.IsNullOrWhiteSpace(value) || value.Length > 255 || value.Contains('='))
        {
            return false;
        }

        foreach (char character in value)
        {
            if (char.IsWhiteSpace(character) || char.IsControl(character))
            {
                return false;
            }
        }

        return true;
    }

    internal static bool TrySaveRuntimeSettings(
        bool enabled,
        string bindAddress,
        string portText,
        out string error)
    {
        EnsureSelectionLoaded();
        if (!int.TryParse(portText.Trim(), NumberStyles.Integer, CultureInfo.InvariantCulture, out int port)
            || port is < MinRuntimePort or > MaxRuntimePort)
        {
            error = $"Network port must be a whole number from {MinRuntimePort} to {MaxRuntimePort}.";
            return false;
        }

        string normalized = bindAddress.Trim();
        if (!IsValidRuntimeBindAddress(normalized))
        {
            error = "Select a valid IP address or hostname.";
            return false;
        }

        bool previousEnabled = _runtimeEnabled;
        int previousPort = _runtimePort;
        string previousBindAddress = _runtimeBindAddress;
        _runtimeEnabled = enabled;
        _runtimePort = port;
        _runtimeBindAddress = normalized;
        if (TrySaveSelection(out error))
        {
            return true;
        }

        _runtimeEnabled = previousEnabled;
        _runtimePort = previousPort;
        _runtimeBindAddress = previousBindAddress;
        return false;
    }

    internal static bool ResetRuntimeSettings(out string error)
    {
        EnsureSelectionLoaded();
        bool previousEnabled = _runtimeEnabled;
        int previousPort = _runtimePort;
        string previousBindAddress = _runtimeBindAddress;
        _runtimeEnabled = true;
        _runtimePort = DefaultRuntimePort;
        _runtimeBindAddress = DefaultRuntimeBindAddress;
        if (TrySaveSelection(out error))
        {
            return true;
        }

        _runtimeEnabled = previousEnabled;
        _runtimePort = previousPort;
        _runtimeBindAddress = previousBindAddress;
        return false;
    }

    private static void EnsureSelectionLoaded()
    {
        if (_selectionLoaded) return;
        _selectionLoaded = true;
        try
        {
            if (!File.Exists(GetSettingsPath()))
            {
                return;
            }

            foreach (string rawLine in File.ReadAllLines(GetSettingsPath()))
            {
                string line = rawLine.Trim();
                if (line.Length == 0) continue;

                int separator = line.IndexOf('=');
                if (separator < 0)
                {
                    // The first shipped settings version stored only the profile number.
                    if (int.TryParse(line, NumberStyles.Integer, CultureInfo.InvariantCulture, out int legacyProfile))
                    {
                        _selectedProfileId = ClampProfileId(legacyProfile);
                    }

                    continue;
                }

                string key = line[..separator].Trim();
                string value = line[(separator + 1)..].Trim();
                switch (key)
                {
                    case "profile_id":
                        if (int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int profileId))
                        {
                            _selectedProfileId = ClampProfileId(profileId);
                        }

                        break;
                    case "runtime_enabled":
                        if (bool.TryParse(value, out bool enabled))
                        {
                            _runtimeEnabled = enabled;
                        }
                        else if (value is "1" or "yes" or "on")
                        {
                            _runtimeEnabled = true;
                        }
                        else if (value is "0" or "no" or "off")
                        {
                            _runtimeEnabled = false;
                        }

                        break;
                    case "runtime_port":
                        if (int.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, out int port)
                            && port is >= MinRuntimePort and <= MaxRuntimePort)
                        {
                            _runtimePort = port;
                        }

                        break;
                    case "runtime_bind_address":
                        if (IsValidRuntimeBindAddress(value))
                        {
                            _runtimeBindAddress = value;
                        }

                        break;
                }
            }
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile setting load failed: {exception.GetType().Name}");
        }
    }

    private static void SaveSelection()
    {
        if (!TrySaveSelection(out string error))
        {
            GD.PrintErr($"{LogPrefix} profile setting save failed: {error}");
        }
    }

    private static bool TrySaveSelection(out string error)
    {
        try
        {
            Directory.CreateDirectory(OS.GetUserDataDir());
            File.WriteAllLines(
                GetSettingsPath(),
                new[]
                {
                    $"profile_id={ClampProfileId(_selectedProfileId).ToString(CultureInfo.InvariantCulture)}",
                    $"runtime_enabled={_runtimeEnabled.ToString().ToLowerInvariant()}",
                    $"runtime_port={_runtimePort.ToString(CultureInfo.InvariantCulture)}",
                    $"runtime_bind_address={_runtimeBindAddress}"
                });
            error = string.Empty;
            return true;
        }
        catch (Exception exception)
        {
            error = exception.GetType().Name;
            return false;
        }
    }

    private static string GetSettingsPath() => Path.Combine(OS.GetUserDataDir(), SettingsFileName);

    private static int ClampProfileId(int profileId) => Math.Clamp(profileId, MinProfileId, MaxProfileId);
}
