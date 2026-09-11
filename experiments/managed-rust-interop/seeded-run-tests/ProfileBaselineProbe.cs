// SPDX-License-Identifier: MIT

using System;
using System.IO;
using AiAscension.Sts2GameMod.Runtime;

namespace Godot
{
    internal static class OS
    {
        internal static string UserDataDir { get; set; } = string.Empty;

        internal static string GetUserDataDir() => UserDataDir;
    }
}

namespace AiAscension.Sts2GameMod.Runtime
{
    internal static class ProfileBaselineProbe
    {
        private static void Main()
        {
            string root = Path.Combine(Path.GetTempPath(),
                "sts2-profile-baseline-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(root);
            Godot.OS.UserDataDir = root;
            try
            {
                CacheTelemetryDoesNotChangeDigest(root);
                PersistentMutationChangesDigest(root, "settings.save", "settings changed",
                    "settings mutation is included");
                PersistentMutationChangesDigest(root, "profiles/profile_1/progress.save",
                    "progress changed", "progress mutation is included");
                PersistentMutationChangesDigest(root, "profiles/profile_1/current_run.save",
                    "current run changed", "current save mutation is included");
                PersistentMutationChangesDigest(root, "profiles/shader_cache/owned.save",
                    "nested persistent file changed", "nested unknown file is included");
                Console.WriteLine("seeded-run profile baseline inventory checks passed");
            }
            finally
            {
                if (Directory.Exists(root))
                {
                    Directory.Delete(root, recursive: true);
                }
            }
        }

        private static void CacheTelemetryDoesNotChangeDigest(string root)
        {
            string caseRoot = NewCase(root, "cache-telemetry");
            Write(caseRoot, "settings.save", "settings");
            Write(caseRoot, "profiles/profile_1/progress.save", "progress");
            Write(caseRoot, "shader_cache/boot.bin", "shader");
            Write(caseRoot, "SENTRY/crash.log", "sentry");
            Write(caseRoot, "sentry.dat", "sentry data");
            Write(caseRoot, "SHADER_CACHE/Case.bin", "case-insensitive shader");
            string digest = SeededRunProfileBaseline.CaptureInitial();

            Write(caseRoot, "shader_cache/after-capture.bin", "new shader");
            Write(caseRoot, "sentry/after-capture.log", "new sentry");
            Write(caseRoot, "SENTRY.DAT", "case-insensitive sentry data");
            string afterTelemetryDigest = SeededRunProfileBaseline.CaptureInitial();
            Check(string.Equals(digest, afterTelemetryDigest, StringComparison.Ordinal),
                "cache and telemetry changes preserve the captured digest");
            Check(Matches(digest), "cache and telemetry changes preserve the baseline digest");
        }

        private static void PersistentMutationChangesDigest(
            string root,
            string relativePath,
            string contents,
            string message)
        {
            string caseRoot = NewCase(root, relativePath.Replace('/', '-'));
            Write(caseRoot, "settings.save", "settings");
            Write(caseRoot, "shader_cache/boot.bin", "shader");
            Write(caseRoot, "sentry/crash.log", "sentry");
            Write(caseRoot, "sentry.dat", "sentry data");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            Write(caseRoot, relativePath, contents);
            string changedDigest = SeededRunProfileBaseline.CaptureInitial();
            Check(!string.Equals(digest, changedDigest, StringComparison.Ordinal),
                message);
            Check(!Matches(digest), message);
        }

        private static bool Matches(string digest)
        {
            SeededRunContextProfileBaseline baseline = new(
                SeededRunSelectionContext.FreshProfileKind,
                "fresh-standard-comparison",
                digest);
            return SeededRunProfileBaseline.Matches(baseline, out _);
        }

        private static string NewCase(string root, string name)
        {
            string caseRoot = Path.Combine(root, name);
            Directory.CreateDirectory(caseRoot);
            Godot.OS.UserDataDir = caseRoot;
            return caseRoot;
        }

        private static void Write(string root, string relativePath, string contents)
        {
            string path = Path.Combine(root, relativePath.Replace('/', Path.DirectorySeparatorChar));
            string? directory = Path.GetDirectoryName(path);
            if (directory is not null)
            {
                Directory.CreateDirectory(directory);
            }

            File.WriteAllText(path, contents);
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
}
