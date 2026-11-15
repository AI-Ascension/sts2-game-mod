// SPDX-License-Identifier: MIT

using System;
using System.IO;

namespace AiAscension.Sts2GameMod.Runtime
{
    internal static partial class ProfileBaselineProbe
    {
        private static void BootDerivedLinkMatrix(string root)
        {
            foreach (string kind in new[] { "sentry-file", "cache-root", "nested-cache-file" })
            {
                BootDerivedLinksCannotBeAdmitted(root, kind, late: false);
                BootDerivedLinksCannotBeAdmitted(root, kind, late: true);
            }
        }

        private static void BootDerivedLinksCannotBeAdmitted(string root, string kind, bool late)
        {
            string caseRoot = NewCase(root, (late ? "late-" : "capture-") + "boot-link-" + kind);
            Write(caseRoot, "settings.save", "original settings");
            if (kind == "nested-cache-file")
                Directory.CreateDirectory(Path.Combine(caseRoot, "shader_cache", "sub"));

            string outside = Path.Combine(root,
                "outside-boot-link-" + kind + (late ? "-late" : "-capture"));
            bool directory = kind == "cache-root";
            if (directory) Write(outside, "foreign.save", "unrelated player data");
            else File.WriteAllText(outside, "unrelated player data");

            string digest = late ? SeededRunProfileBaseline.CaptureInitial() : string.Empty;
            string linkPath = kind switch
            {
                "sentry-file" => Path.Combine(caseRoot, "sentry.dat"),
                "cache-root" => Path.Combine(caseRoot, "shader_cache"),
                _ => Path.Combine(caseRoot, "shader_cache", "sub", "foreign.save")
            };
            if (!TryCreateLink(() =>
            {
                if (directory) Directory.CreateSymbolicLink(linkPath, outside);
                else File.CreateSymbolicLink(linkPath, outside);
            })) return;

            string label = (late ? "late " : "initial ") + kind;
            if (late)
            {
                Check(!Matches(digest), label + " linked cache/telemetry refuses admission");
                Check(!Matches(digest), label + " refusal clears cached baseline");
            }
            else
            {
                bool refused = false;
                try { SeededRunProfileBaseline.CaptureInitial(); }
                catch (InvalidOperationException error)
                    when (error.Message.StartsWith("isolated user directory", StringComparison.Ordinal))
                {
                    refused = true;
                }
                Check(refused, label + " linked cache/telemetry refuses capture");
            }

            string outsideFile = directory ? Path.Combine(outside, "foreign.save") : outside;
            Check(File.ReadAllText(outsideFile) == "unrelated player data",
                label + " refusal preserves unrelated data");
        }

        private static void LateLinkedEntryAddedAfterCaptureIsRefused(string root, bool directory)
        {
            string suffix = directory ? "directory" : "file";
            string caseRoot = NewCase(root, "late-linked-" + suffix);
            Write(caseRoot, "settings.save", "original settings");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            string outside = Path.Combine(root, "late-outside-" + suffix);
            if (directory) Write(outside, "save.dat", "unrelated player data");
            else File.WriteAllText(outside, "unrelated player data");
            if (!TryCreateLink(() =>
            {
                if (directory) Directory.CreateSymbolicLink(Path.Combine(caseRoot, "foreign"), outside);
                else File.CreateSymbolicLink(Path.Combine(caseRoot, "foreign.save"), outside);
            })) return;

            Check(!Matches(digest), "late linked " + suffix + " refuses admission");
            Check(!Matches(digest), "late linked " + suffix + " clears cached evidence");
            string outsideFile = directory ? Path.Combine(outside, "save.dat") : outside;
            Check(File.ReadAllText(outsideFile) == "unrelated player data",
                "late linked " + suffix + " preserves unrelated data");
        }

        private static void LinkedAncestorIsNotProfileEvidence(string root)
        {
            string realParent = Path.Combine(root, "real-profile-ancestor");
            string originalProfile = Path.Combine(realParent, "profile");
            Write(originalProfile, "settings.save", "original profile");
            Godot.OS.UserDataDir = originalProfile;
            string digest = SeededRunProfileBaseline.CaptureInitial();
            string alias = Path.Combine(root, "linked-profile-ancestor");
            if (!TryCreateLink(() => Directory.CreateSymbolicLink(alias, realParent))) return;
            try
            {
                Godot.OS.UserDataDir = Path.Combine(alias, "profile");
                CheckRejectedAndCleared(digest,
                    "symlinked ancestor refuses an otherwise unchanged original profile");
                Check(File.ReadAllText(Path.Combine(originalProfile, "settings.save"))
                    == "original profile", "ancestor refusal preserves original profile bytes");
            }
            finally
            {
                Directory.Delete(alias);
            }
        }

        private static void ReplacedAncestorWithIdenticalLinkedTreeIsRefused(string root)
        {
            string ancestor = Path.Combine(root, "late-profile-ancestor");
            string profile = Path.Combine(ancestor, "profile");
            Write(profile, "settings.save", "original profile");
            Godot.OS.UserDataDir = profile;
            string digest = SeededRunProfileBaseline.CaptureInitial();
            string moved = ancestor + "-original";
            Directory.Move(ancestor, moved);
            if (!TryCreateLink(() => Directory.CreateSymbolicLink(ancestor, moved)))
            {
                Directory.Move(moved, ancestor);
                return;
            }
            try
            {
                // Original bytes and lexical profile path are unchanged. The parent
                // became a link after capture; that must invalidate admission.
                Check(!Matches(digest), "late symlinked ancestor refuses admission");
                Check(!Matches(digest), "late ancestor refusal clears prior evidence");
                Check(File.ReadAllText(Path.Combine(moved, "profile", "settings.save"))
                    == "original profile", "late ancestor refusal preserves original bytes");
            }
            finally
            {
                Directory.Delete(ancestor);
                Directory.Move(moved, ancestor);
            }
        }

    }
}
