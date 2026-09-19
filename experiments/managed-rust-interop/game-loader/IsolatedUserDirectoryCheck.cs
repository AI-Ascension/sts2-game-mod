// SPDX-License-Identifier: MIT

using System;
using System.IO;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// The isolated-user-directory precondition the production live runtime enforces before it
/// replaces the save backend.
///
/// The game decides the user directory it writes to (Godot's <c>OS.GetUserDataDir</c>, steered by
/// <c>override.cfg</c> and the launch-scoped APPDATA root), and the launcher declares the directory
/// it believes it isolated in <c>STS2_LIVE_USER_DIR</c>. Refusing when those disagree is correct.
/// Refusing without naming the directory the game examined, the value the launcher declared, or
/// which of the two was missing is not: it produced an investigation that could not be acted on
/// (sts2-game-mod#173).
///
/// This type is deliberately host-independent. It depends only on the base class library, so the
/// same decision compiles into the source-only managed probe that runs without a game.
/// </summary>
internal static class IsolatedUserDirectoryCheck
{
    /// <summary>Stable prefix an operator already recognizes in <c>game.log</c>.</summary>
    internal const string Refusal = "live demo requires its isolated user directory";

    /// <summary>The launcher declared no directory, so agreement cannot be established.</summary>
    internal const string ExpectedUnset = "isolated_user_dir_unset";

    /// <summary>The game resolved no directory, so there is nothing to isolate.</summary>
    internal const string ResolvedUnset = "isolated_user_dir_unresolved";

    /// <summary>The game and the launcher named different directories.</summary>
    internal const string Mismatch = "isolated_user_dir_mismatch";

    /// <summary>
    /// Classifies the precondition. The returned diagnostic always names the directory the game
    /// resolved, the directory the launcher declared, and the stable reason token, so a failing
    /// launch states which condition failed instead of only that one did.
    /// </summary>
    internal static IsolatedUserDirectoryDecision Evaluate(
        string? resolvedUserDataDir,
        string? declaredUserDir)
    {
        string resolved = Normalize(resolvedUserDataDir);
        string declared = Normalize(declaredUserDir);

        if (declared.Length == 0)
        {
            return new IsolatedUserDirectoryDecision(false, ExpectedUnset,
                $"{Refusal}: STS2_LIVE_USER_DIR is unset, but the game resolved its user directory "
                + $"to '{Describe(resolved)}'. Point the launcher's STS2_LIVE_USER_DIR at that "
                + "directory, or set override.cfg's config/custom_user_dir_name so the game "
                + "resolves the directory you isolated.");
        }

        if (resolved.Length == 0)
        {
            return new IsolatedUserDirectoryDecision(false, ResolvedUnset,
                $"{Refusal}: the game resolved no user directory, but STS2_LIVE_USER_DIR declared "
                + $"'{declared}'.");
        }

        if (!string.Equals(resolved, declared, StringComparison.OrdinalIgnoreCase))
        {
            return new IsolatedUserDirectoryDecision(false, Mismatch,
                $"{Refusal}: the game resolved its user directory to '{resolved}', but "
                + $"STS2_LIVE_USER_DIR declared '{declared}'. config/custom_user_dir_name in "
                + "override.cfg and the launch-scoped APPDATA root decide the resolved path.");
        }

        return new IsolatedUserDirectoryDecision(true, string.Empty, string.Empty);
    }

    private static string Normalize(string? value)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            return string.Empty;
        }

        string trimmed = value.Trim();
        try
        {
            return CollapseTrailingSeparators(Path.GetFullPath(trimmed));
        }
        catch (ArgumentException)
        {
            return trimmed;
        }
        catch (NotSupportedException)
        {
            return trimmed;
        }
        catch (PathTooLongException)
        {
            return trimmed;
        }
    }

    /// <summary>
    /// Absolute-form normalization, except that trailing separators are removed on every platform.
    /// <see cref="Path.GetFullPath(string)"/> keeps a trailing separator on Unix and removes it on
    /// Windows, which would otherwise make the same declared directory agree on one platform and
    /// not another.
    /// </summary>
    private static string CollapseTrailingSeparators(string full)
    {
        string root = Path.GetPathRoot(full) ?? string.Empty;
        return full.Length > root.Length
            ? full.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
            : full;
    }

    private static string Describe(string value) => value.Length == 0 ? "(none)" : value;
}

/// <summary>Outcome of the isolated-user-directory precondition.</summary>
internal readonly record struct IsolatedUserDirectoryDecision(
    bool Agreed,
    string Reason,
    string Diagnostic);
