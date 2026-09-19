// SPDX-License-Identifier: MIT

using System;
using System.IO;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Source-only checks for the isolated-user-directory precondition. The production live runtime
/// refuses to replace the save backend unless the game's resolved user directory agrees with the
/// launcher's declaration; before sts2-game-mod#173 the refusal did not say which directory was
/// examined or which condition failed.
/// </summary>
internal static class IsolatedUserDirectoryProbe
{
    private static void Main()
    {
        string isolated = Path.Combine(Path.GetTempPath(), "sts2-isolated-user-dir");
        string other = Path.Combine(Path.GetTempPath(), "sts2-shared-user-dir");

        IsolatedUserDirectoryDecision agreed =
            IsolatedUserDirectoryCheck.Evaluate(isolated, isolated);
        Check(agreed.Agreed, "the declared directory is admitted when the game resolves it");
        Check(agreed.Reason.Length == 0 && agreed.Diagnostic.Length == 0,
            "an admitted directory carries no refusal reason or diagnostic");

        Check(IsolatedUserDirectoryCheck.Evaluate(isolated, isolated.ToUpperInvariant()).Agreed,
            "agreement is case-insensitive");
        Check(IsolatedUserDirectoryCheck.Evaluate(
                isolated + Path.DirectorySeparatorChar, isolated).Agreed,
            "agreement tolerates a trailing separator");

        foreach (string? unset in new string?[] { null, "", "   " })
        {
            IsolatedUserDirectoryDecision decision =
                IsolatedUserDirectoryCheck.Evaluate(isolated, unset);
            Check(!decision.Agreed
                    && decision.Reason == IsolatedUserDirectoryCheck.ExpectedUnset
                    && decision.Diagnostic.StartsWith(
                        IsolatedUserDirectoryCheck.Refusal, StringComparison.Ordinal)
                    && decision.Diagnostic.Contains(isolated, StringComparison.Ordinal),
                "an unset declaration names the directory the game resolved");
        }

        IsolatedUserDirectoryDecision unresolved =
            IsolatedUserDirectoryCheck.Evaluate("", isolated);
        Check(!unresolved.Agreed
                && unresolved.Reason == IsolatedUserDirectoryCheck.ResolvedUnset
                && unresolved.Diagnostic.Contains(isolated, StringComparison.Ordinal),
            "an unresolved game directory names the declared value");

        IsolatedUserDirectoryDecision mismatched =
            IsolatedUserDirectoryCheck.Evaluate(isolated, other);
        Check(!mismatched.Agreed && mismatched.Reason == IsolatedUserDirectoryCheck.Mismatch,
            "a different declared directory is refused");
        Check(mismatched.Diagnostic.StartsWith(
                IsolatedUserDirectoryCheck.Refusal, StringComparison.Ordinal)
                && mismatched.Diagnostic.Contains(isolated, StringComparison.Ordinal)
                && mismatched.Diagnostic.Contains(other, StringComparison.Ordinal),
            "a mismatch names both the resolved and the declared directory");
        Check(mismatched.Diagnostic.Contains("override.cfg", StringComparison.Ordinal),
            "a mismatch names the override that decides the resolved directory");

        Check(IsolatedUserDirectoryCheck.ExpectedUnset != IsolatedUserDirectoryCheck.ResolvedUnset
                && IsolatedUserDirectoryCheck.ResolvedUnset != IsolatedUserDirectoryCheck.Mismatch
                && IsolatedUserDirectoryCheck.Mismatch != IsolatedUserDirectoryCheck.ExpectedUnset,
            "each refusal carries its own stable reason token");

        Console.WriteLine("isolated user directory checks passed");
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }

        Console.WriteLine("PASS: " + message);
    }
}
