// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

// This is the deliberately separate mutating gate for one new Workshop item.
// It creates an empty item only. Upload and update remain SteamCMD operations
// guarded by tools/workshop/lifecycle/workshop_operator.

internal static partial class Program
{
    private sealed class Options
    {
        internal required string LibraryPath { get; init; }
        internal required string ResolvedLibraryPath { get; init; }
        internal required string ExpectedLibrarySha256 { get; init; }
        internal required string PackageDirectory { get; init; }
        internal required string Platform { get; init; }
        internal required string OutputPath { get; init; }
        internal required string JournalPath { get; init; }
        internal required uint ExpectedAppId { get; init; }
        internal required TimeSpan Timeout { get; init; }

        internal static Options Parse(string[] args)
        {
            string? library = null;
            string? expectedSha = null;
            string? package = null;
            string? platform = null;
            string? output = null;
            string? journal = null;
            uint appId = 2868840;
            int timeout = 120;
            for (int index = 0; index < args.Length; index++)
            {
                string value = args[index];
                if (index + 1 < args.Length)
                {
                    switch (value)
                    {
                        case "--library": library = args[++index]; continue;
                        case "--expected-sha256": expectedSha = args[++index]; continue;
                        case "--package-dir": package = args[++index]; continue;
                        case "--platform": platform = args[++index]; continue;
                        case "--output": output = args[++index]; continue;
                        case "--journal": journal = args[++index]; continue;
                        case "--app-id" when uint.TryParse(args[++index], out uint parsedApp): appId = parsedApp; continue;
                        case "--timeout-seconds" when int.TryParse(args[++index], out int parsedTimeout): timeout = parsedTimeout; continue;
                    }
                }
                if (value is "--help" or "-h")
                {
                    throw new UsageException("usage: Sts2.UgcCreateItemHelper --library LIB --expected-sha256 SHA --package-dir PACKAGE --platform linux-x86_64 --output RESULT --journal JOURNAL [--app-id 2868840] [--timeout-seconds 120]");
                }
                throw new UsageException("invalid arguments; use --help");
            }
            if (string.IsNullOrWhiteSpace(library) || string.IsNullOrWhiteSpace(expectedSha)
                || string.IsNullOrWhiteSpace(package) || string.IsNullOrWhiteSpace(platform)
                || string.IsNullOrWhiteSpace(output) || string.IsNullOrWhiteSpace(journal))
            {
                throw new UsageException("--library, --expected-sha256, --package-dir, --platform, --output, and --journal are required");
            }
            if (platform != "linux-x86_64") throw new UsageException("unsupported platform; the verified CreateItem ABI is Linux x86-64 only");
            if (!OperatingSystem.IsLinux() || RuntimeInformation.ProcessArchitecture != Architecture.X64)
                throw new UsageException("the verified CreateItem helper requires Linux x86-64");
            if (!IsSha256(expectedSha)) throw new UsageException("--expected-sha256 must be 64 hexadecimal characters");
            if (appId == 0 || timeout is < 1 or > MaximumTimeoutSeconds) throw new UsageException("app ID or timeout is outside its bound");
            string libraryPath = Path.GetFullPath(library);
            string packagePath = Path.GetFullPath(package);
            string outputPath = Path.GetFullPath(output);
            string journalPath = Path.GetFullPath(journal);
            if (!File.Exists(libraryPath) || new FileInfo(libraryPath).LinkTarget is not null) throw new UsageException("library must be an existing regular non-symlink file");
            if (!Directory.Exists(packagePath) || new DirectoryInfo(packagePath).LinkTarget is not null) throw new UsageException("package directory must be an existing regular non-symlink directory");
            if (IsWithin(packagePath, outputPath) || IsWithin(packagePath, journalPath) || outputPath == journalPath)
                throw new UsageException("output and journal must be outside the package directory and differ");
            string resolved = new FileInfo(libraryPath).ResolveLinkTarget(true)?.FullName ?? libraryPath;
            return new Options
            {
                LibraryPath = libraryPath,
                ResolvedLibraryPath = resolved,
                ExpectedLibrarySha256 = expectedSha.ToLowerInvariant(),
                PackageDirectory = packagePath,
                Platform = platform,
                OutputPath = outputPath,
                JournalPath = journalPath,
                ExpectedAppId = appId,
                Timeout = TimeSpan.FromSeconds(timeout)
            };
        }

        private static bool IsWithin(string parent, string candidate)
        {
            string prefix = parent.TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar;
            return candidate.StartsWith(prefix, StringComparison.Ordinal) || candidate == parent;
        }
    }

    private sealed class UsageException(string message) : Exception(message);

    private sealed class PackageSnapshot
    {
        public required string ManifestSha256 { get; init; }
        public required string ContentDigest { get; init; }
        public required PackageFile[] Files { get; init; }
    }

    private sealed class PackageFile
    {
        public required string Path { get; init; }
        public required string Role { get; init; }
        public required long SizeBytes { get; init; }
        public required string Sha256 { get; init; }
    }

    private sealed class Journal
    {
        public string? Schema { get; set; }
        public string? Phase { get; set; }
        public string? Outcome { get; set; }
        public uint ExpectedAppId { get; set; }
        public uint ActualAppId { get; set; }
        public string? Platform { get; set; }
        public string? PackageDirectory { get; set; }
        public string? PackageManifestSha256 { get; set; }
        public string? PackageContentDigest { get; set; }
        public PackageFile[]? PackageFiles { get; set; }
        public string? LibraryPath { get; set; }
        public string? ResolvedLibraryPath { get; set; }
        public string? LibrarySha256 { get; set; }
        public bool AppEnvironmentMatches { get; set; }
        public bool AppIdMatches { get; set; }
        public bool LoggedOn { get; set; }
        public bool Subscribed { get; set; }
        public string? CallHandle { get; set; }
        public int? ResultCode { get; set; }
        public ulong? PublishedFileId { get; set; }
        public bool UserNeedsLegalAgreement { get; set; }
        public string? ErrorType { get; set; }
        public string? JournalPath { get; set; }
        public DateTimeOffset UpdatedAtUtc { get; set; }
    }

    private sealed class Result
    {
        public string? Schema { get; set; }
        public uint ExpectedAppId { get; set; }
        public uint AppId { get; set; }
        public bool AppIdMatches { get; set; }
        public bool AppEnvironmentMatches { get; set; }
        public string? Platform { get; set; }
        public string? PackageDirectory { get; set; }
        public bool PackagePreflightPassed { get; set; }
        public string? PackageManifestSha256 { get; set; }
        public string? PackageContentDigest { get; set; }
        public PackageFile[]? PackageFiles { get; set; }
        public string? LibraryPath { get; set; }
        public string? ResolvedLibraryPath { get; set; }
        public string? ExpectedLibrarySha256 { get; set; }
        public string? LibrarySha256 { get; set; }
        public bool LibraryHashMatches { get; set; }
        public bool NativeLibraryLoaded { get; set; }
        public string? LoadedLibraryPath { get; set; }
        public bool LoadedLibraryMatches { get; set; }
        public bool InitAttempted { get; set; }
        public int InitResult { get; set; }
        public string? InitFailureCategory { get; set; }
        public bool InitFailureDiagnosticPresent { get; set; }
        public bool AppsInterfaceAvailable { get; set; }
        public bool UtilsInterfaceAvailable { get; set; }
        public bool UserInterfaceAvailable { get; set; }
        public bool UgcInterfaceAvailable { get; set; }
        public bool LoggedOn { get; set; }
        public bool Subscribed { get; set; }
        public bool JournalCreated { get; set; }
        public string? JournalPath { get; set; }
        public bool CreateItemCallIssued { get; set; }
        public bool ApiCallCompleted { get; set; }
        public bool ApiCallFailed { get; set; }
        public bool GetApiCallResultSucceeded { get; set; }
        public bool ResultRetrievalFailed { get; set; }
        public int ResultCode { get; set; }
        public ulong PublishedFileId { get; set; }
        public bool UserNeedsLegalAgreement { get; set; }
        public string Outcome { get; set; } = "unknown";
        public string? ErrorType { get; set; }

        internal int ExitCode(Options options)
            => ErrorType is null
                && PackagePreflightPassed
                && LibraryHashMatches
                && LoadedLibraryMatches
                && AppEnvironmentMatches
                && InitResult == 0
                && AppIdMatches
                && LoggedOn
                && Subscribed
                && UgcInterfaceAvailable
                && JournalCreated
                && CreateItemCallIssued
                && ApiCallCompleted
                && !ApiCallFailed
                && GetApiCallResultSucceeded
                && !ResultRetrievalFailed
                && ResultCode == EResultOk
                && PublishedFileId != 0
                && Outcome == "succeeded"
                ? 0
                : 2;
    }
}
