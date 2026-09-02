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
    private static bool _selectionLoaded;
    private static int _selectedProfileId = DefaultProfileId;

    private static void EnsureSelectionLoaded()
    {
        if (_selectionLoaded) return;
        _selectionLoaded = true;
        try
        {
            if (File.Exists(GetSettingsPath())
                && int.TryParse(File.ReadAllText(GetSettingsPath()).Trim(), out int profileId))
            {
                _selectedProfileId = ClampProfileId(profileId);
            }
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile setting load failed: {exception.GetType().Name}");
        }
    }

    private static void SaveSelection()
    {
        try
        {
            Directory.CreateDirectory(OS.GetUserDataDir());
            File.WriteAllText(
                GetSettingsPath(),
                ClampProfileId(_selectedProfileId).ToString(CultureInfo.InvariantCulture));
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} profile setting save failed: {exception.GetType().Name}");
        }
    }

    private static string GetSettingsPath() => Path.Combine(OS.GetUserDataDir(), SettingsFileName);

    private static int ClampProfileId(int profileId) => Math.Clamp(profileId, MinProfileId, MaxProfileId);
}
