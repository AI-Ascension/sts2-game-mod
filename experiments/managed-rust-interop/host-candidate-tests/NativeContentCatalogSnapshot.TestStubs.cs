// SPDX-License-Identifier: MIT

namespace MegaCrit.Sts2.Core.Debug
{
    internal sealed class ReleaseInfo
    {
        internal string? Version { get; init; }
    }

    internal sealed class ReleaseInfoManager
    {
        internal static ReleaseInfoManager? Instance { get; set; }
        internal ReleaseInfo? ReleaseInfo { get; init; }
    }
}

namespace MegaCrit.Sts2.Core.Localization
{
    internal sealed class LocManager
    {
        internal static LocManager? Instance { get; set; }
        internal string Language { get; init; } = string.Empty;
    }
}

namespace MegaCrit.Sts2.Core.Modding
{
    internal sealed class ModManifest
    {
        internal string? id;
        internal string? version;
    }

    internal sealed class Mod
    {
        internal ModManifest? manifest;
    }

    internal static class ModManager
    {
        internal static IEnumerable<Mod> LoadedMods { get; set; } = Array.Empty<Mod>();
        internal static IEnumerable<Mod> GetLoadedMods() => LoadedMods;
    }
}
