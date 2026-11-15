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

    }
}
