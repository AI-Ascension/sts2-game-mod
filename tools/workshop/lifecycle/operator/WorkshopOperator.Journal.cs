// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

internal static partial class Program
{
    private static OperationJournal NewJournal(
        string operation,
        uint appId,
        ulong itemId,
        string journalPath,
        string[] command,
        PackageInfo? package,
        string? platform,
        string? expectedInstallDirectory,
        Dictionary<string, string>? environment)
        => new()
        {
            Operation = operation,
            AppId = appId,
            PublishedFileId = itemId,
            Package = package,
            Command = command,
            Journal = journalPath,
            Platform = platform,
            ExpectedInstallDirectory = expectedInstallDirectory,
            Environment = environment,
            Phase = "prepared",
            Outcome = "not_started",
            UpdatedAtUtc = UtcNow()
        };

    private static int RunJournaled(
        string operation,
        uint appId,
        ulong itemId,
        string journalPath,
        string[] command,
        PackageInfo? package,
        string? platform,
        string? expectedInstallDirectory,
        Dictionary<string, string>? environment,
        bool execute,
        int timeout,
        Action? postcondition)
    {
        OperationJournal plan = NewJournal(
            operation,
            appId,
            itemId,
            journalPath,
            command,
            package,
            platform,
            expectedInstallDirectory,
            environment);
        if (!execute)
        {
            Console.WriteLine(Serialize(new
            {
                plan.Schema,
                plan.Operation,
                plan.AppId,
                plan.PublishedFileId,
                plan.Package,
                plan.Command,
                plan.Journal,
                plan.Platform,
                plan.ExpectedInstallDirectory,
                plan.Environment,
                plan.Phase,
                plan.Outcome,
                plan.UpdatedAtUtc,
                ExecuteRequired = true
            }));
            return 0;
        }
        if (timeout is < 1 or > 3600)
            throw new OperatorException("timeout must be between one and 3600 seconds");
        string steamUser = "";
        if (operation is "upload" or "update" or "rollback" or "download")
        {
            steamUser = Environment.GetEnvironmentVariable("STEAMCMD_USER") ?? "";
            if (steamUser.Length == 0)
                throw new OperatorException(
                    "STEAMCMD_USER must be set for SteamCMD execution; no password is accepted");
        }
        string journal = EnsureAbsent(journalPath, "operation journal");
        string log = EnsureAbsent($"{journal}.log", "operation log");
        string[] childCommand = command
            .Select(value => value == "<STEAMCMD_USER>" ? steamUser : value)
            .ToArray();
        Directory.CreateDirectory(Path.GetDirectoryName(journal) ?? ".");
        WriteJsonExclusive(journal, plan);
        try
        {
            int? exitCode;
            bool timedOut;
            using (FileStream logStream = CreatePrivateFile(log))
            {
                (exitCode, timedOut) = RunProcess(childCommand, environment, timeout, logStream);
            }
            if (timedOut)
            {
                plan.Phase = "unknown_timeout";
                plan.Outcome = "unknown";
                plan.UpdatedAtUtc = UtcNow();
                WriteJsonAtomic(journal, plan);
                Console.WriteLine(Serialize(plan));
                return 2;
            }
            plan.ProcessExitCode = exitCode;
            if (exitCode is null or not 0)
            {
                plan.Phase = "unknown_process_exit";
                plan.Outcome = "unknown";
                plan.UpdatedAtUtc = UtcNow();
                WriteJsonAtomic(journal, plan);
                Console.WriteLine(Serialize(plan));
                return 2;
            }
            if (postcondition is not null)
            {
                try
                {
                    postcondition();
                }
                catch (Exception exception)
                {
                    plan.Phase = "unknown_postcondition";
                    plan.Outcome = "unknown";
                    plan.PostconditionError = exception.Message;
                    plan.UpdatedAtUtc = UtcNow();
                    WriteJsonAtomic(journal, plan);
                    Console.WriteLine(Serialize(plan));
                    return 2;
                }
            }
            plan.Phase = "succeeded";
            plan.Outcome = "succeeded";
            plan.UpdatedAtUtc = UtcNow();
            WriteJsonAtomic(journal, plan);
            Console.WriteLine(Serialize(plan));
            return 0;
        }
        catch (Exception exception)
        {
            plan.Phase = "unknown_exception";
            plan.Outcome = "unknown";
            plan.ExceptionType = exception.GetType().Name;
            plan.UpdatedAtUtc = UtcNow();
            WriteJsonAtomic(journal, plan);
            Console.WriteLine(Serialize(plan));
            return 2;
        }
    }
}
