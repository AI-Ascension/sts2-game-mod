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
    private const string Schema = "sts2-steam-ugc-create-item-helper-v2";
    private const string PackageSchema = "sts2-workshop-manifest-v1";
    private const string PackageId = "ai-ascension.sts2-game-mod";
    private const string LoaderContract = "sts2-managed-loader-v1";
    private const string Entrypoint = "AIAscensionSTS2GameMod.json";
    private const int CreateItemCallback = 3403;
    private const int CommunityFileType = 0;
    private const int EResultOk = 1;
    private const int CreateItemResultSize = 16;
    private const int ErrorBufferCapacity = 4096;
    private const int MaximumTimeoutSeconds = 300;

    private static int Main(string[] args)
    {
        Options options;
        try
        {
            options = Options.Parse(args);
        }
        catch (UsageException exception)
        {
            Console.Error.WriteLine(exception.Message);
            return 64;
        }

        Result result;
        try
        {
            result = Run(options);
            WriteJson(options.OutputPath, result);
        }
        catch (Exception exception)
        {
            result = new Result
            {
                Schema = Schema,
                ExpectedAppId = options.ExpectedAppId,
                Platform = options.Platform,
                PackageDirectory = options.PackageDirectory,
                LibraryPath = options.LibraryPath,
                JournalPath = options.JournalPath,
                Outcome = "unknown",
                ErrorType = exception.GetType().Name
            };
            try
            {
                WriteJson(options.OutputPath, result);
            }
            catch
            {
                // The process exit code remains useful when the output path is unavailable.
            }
        }

        Console.WriteLine($"UGC CreateItem result written to {options.OutputPath}");
        return result.ExitCode(options);
    }

}
