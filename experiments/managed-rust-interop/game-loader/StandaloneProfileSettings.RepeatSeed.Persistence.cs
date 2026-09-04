// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class StandaloneProfileSettings
{
    private static bool _allowRepeatingSeeds;

    internal static bool AllowRepeatingSeeds
    {
        get
        {
            EnsureSelectionLoaded();
            return _allowRepeatingSeeds;
        }
    }

    internal static bool TrySaveAllowRepeatingSeeds(bool enabled, out string error)
    {
        EnsureSelectionLoaded();
        bool previous = _allowRepeatingSeeds;
        _allowRepeatingSeeds = enabled;
        if (TrySaveSelection(out error))
        {
            return true;
        }

        _allowRepeatingSeeds = previous;
        return false;
    }
}
