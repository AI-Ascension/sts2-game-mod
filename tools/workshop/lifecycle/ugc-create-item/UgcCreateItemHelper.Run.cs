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
    private static Result Run(Options options)
    {
        VerifyCreateItemResultLayout();
        Result result = new()
        {
            Schema = Schema,
            ExpectedAppId = options.ExpectedAppId,
            Platform = options.Platform,
            PackageDirectory = options.PackageDirectory,
            LibraryPath = options.LibraryPath,
            ResolvedLibraryPath = options.ResolvedLibraryPath,
            ExpectedLibrarySha256 = options.ExpectedLibrarySha256,
            JournalPath = options.JournalPath,
            Outcome = "not_started"
        };

        if (PathExists(options.OutputPath))
        {
            result.ErrorType = "OutputExists";
            return result;
        }

        if (PathExists(options.JournalPath))
        {
            result.ErrorType = "JournalExists";
            return result;
        }

        PackageSnapshot? package = VerifyPackage(options);
        if (package is null)
        {
            result.ErrorType = "PackagePreflightFailed";
            return result;
        }

        result.PackagePreflightPassed = true;
        result.PackageManifestSha256 = package.ManifestSha256;
        result.PackageContentDigest = package.ContentDigest;
        result.PackageFiles = package.Files;
        using (FileStream stream = File.OpenRead(options.LibraryPath))
        {
            result.LibrarySha256 = Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
        }

        result.LibraryHashMatches = string.Equals(
            result.LibrarySha256,
            options.ExpectedLibrarySha256,
            StringComparison.OrdinalIgnoreCase);
        if (!result.LibraryHashMatches)
        {
            result.ErrorType = "LibraryHashMismatch";
            return result;
        }

        result.AppEnvironmentMatches = HasExpectedEnvironment(options.ExpectedAppId);
        if (!result.AppEnvironmentMatches)
        {
            result.ErrorType = "AppEnvironmentPreconditionFailed";
            return result;
        }

        return RunNative(options, result, package);
    }
}
