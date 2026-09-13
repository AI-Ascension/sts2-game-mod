// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Pure admission rules for the bounded native standard seeded-run path.
/// This deliberately validates only request-owned context. Host/profile state is
/// checked by <see cref="SeededRunStandardHost"/> on the Godot host thread.
/// </summary>
internal static class SeededRunStandardAdmission
{
    internal const string StandardDefaultSelectionPolicy = "standard_default";

    internal static bool HasSupportedStaticContext(
        SeededRunSelectionContext context,
        out string error)
    {
        if (context.ProfileBaseline.Kind != SeededRunSelectionContext.FreshProfileKind
            || context.SavePolicy != SeededRunSelectionContext.EnabledSavePolicy
            || context.SelectionPolicy != StandardDefaultSelectionPolicy
            || context.Ascension != 0
            || context.Modifiers.Count != 0)
        {
            error = "unsupported_standard_context";
            return false;
        }

        error = string.Empty;
        return true;
    }
}
