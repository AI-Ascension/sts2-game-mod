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
                OriginalProfileChangesAfterCaptureAreRefused(root);
                OverlayOnlyWritesPreserveOriginalProfile(root);
                FailedCaptureInvalidatesPreviousBaseline(root);
                LinkedFileIsNotProfileEvidence(root);
                LinkedDirectoryIsNotProfileEvidence(root);
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

        private static void OriginalProfileChangesAfterCaptureAreRefused(string root)
        {
            string caseRoot = NewCase(root, "late-original-mutation");
            Write(caseRoot, "settings.save", "original settings");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            Write(caseRoot, "settings.save", "changed after capture, before admission");
            Check(!Matches(digest), "original profile mutation after capture refuses admission");
            Check(!Matches(digest), "changed original invalidates cached admission evidence");
        }

        private static void OverlayOnlyWritesPreserveOriginalProfile(string root)
        {
            string caseRoot = NewCase(root, "new-overlay-only");
            Write(caseRoot, "settings.save", "original settings");
            Write(caseRoot, "profiles/profile_1/progress.save", "original progress");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            Write(caseRoot, "live-campaign/settings.save", "new isolated backend settings");
            Write(caseRoot, "live-campaign/progress.save", "new isolated backend progress");
            Write(caseRoot, "sentry/runtime.log", "runtime telemetry");
            Check(Matches(digest), "overlay-only writes preserve original profile witness");
        }

        private static void FailedCaptureInvalidatesPreviousBaseline(string root)
        {
            string caseRoot = NewCase(root, "missing-root");
            Write(caseRoot, "settings.save", "settings");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            Godot.OS.UserDataDir = Path.Combine(root, "missing-after-good-capture");
            CheckRejectedAndCleared(digest, "missing root refuses a stale profile digest");
        }

        private static void LinkedFileIsNotProfileEvidence(string root)
        {
            string caseRoot = NewCase(root, "linked-file");
            Write(caseRoot, "settings.save", "settings");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            string outside = Path.Combine(root, "outside-profile-save.txt");
            File.WriteAllText(outside, "unrelated player data");
            if (!TryCreateLink(() =>
                File.CreateSymbolicLink(Path.Combine(caseRoot, "foreign.save"), outside)))
            {
                return;
            }

            CheckRejectedAndCleared(digest, "linked file refuses unrelated player data");
            Check(File.ReadAllText(outside) == "unrelated player data",
                "linked-file rejection preserves unrelated data");
        }

        private static void LinkedDirectoryIsNotProfileEvidence(string root)
        {
            string caseRoot = NewCase(root, "linked-directory");
            Write(caseRoot, "settings.save", "settings");
            string digest = SeededRunProfileBaseline.CaptureInitial();
            string outside = Path.Combine(root, "outside-profile-directory");
            Write(outside, "save.dat", "unrelated player data");
            if (!TryCreateLink(() =>
                Directory.CreateSymbolicLink(Path.Combine(caseRoot, "foreign"), outside)))
            {
                return;
            }

            CheckRejectedAndCleared(digest, "linked directory refuses unrelated player data");
            Check(File.ReadAllText(Path.Combine(outside, "save.dat")) == "unrelated player data",
                "linked-directory rejection preserves unrelated data");
        }

        private static bool TryCreateLink(Action create)
        {
            try
            {
                create();
                return true;
            }
            catch (UnauthorizedAccessException) when (OperatingSystem.IsWindows())
            {
                Console.WriteLine("SKIP: Windows runner cannot create test-only symbolic links");
                return false;
            }
            catch (IOException) when (OperatingSystem.IsWindows())
            {
                Console.WriteLine("SKIP: Windows runner cannot create test-only symbolic links");
                return false;
            }
            catch (PlatformNotSupportedException)
            {
                Console.WriteLine("SKIP: symbolic links unsupported on this test host");
                return false;
            }
        }

        private static void CheckRejectedAndCleared(string digest, string label)
        {
            bool refused = false;
            try
            {
                SeededRunProfileBaseline.CaptureInitial();
            }
            catch (InvalidOperationException error)
                when (error.Message.StartsWith("isolated user directory", StringComparison.Ordinal))
            {
                refused = true;
            }

            Check(refused, label);
            Check(!Matches(digest), label + " clears previously captured evidence");
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
