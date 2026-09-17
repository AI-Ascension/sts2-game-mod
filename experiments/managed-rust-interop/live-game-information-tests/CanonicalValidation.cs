// SPDX-License-Identifier: MIT

using System;
using System.Diagnostics;
using System.IO;

static class CanonicalValidation
{
    internal static void ValidateCanonicalResponse(string response, string message)
    {
        string? configured = Environment.GetEnvironmentVariable("GAME_INFORMATION_SCHEMA_VALIDATOR");
        string fallback = Path.Combine(Environment.CurrentDirectory, "target", "debug",
            OperatingSystem.IsWindows() ? "game_information_schema_validate.exe" : "game_information_schema_validate");
        string? validator = configured ?? (File.Exists(fallback) ? fallback : null);
        if (validator is null)
            return;
        using var process = Process.Start(new ProcessStartInfo
        {
            FileName = validator,
            RedirectStandardInput = true,
            RedirectStandardError = true,
            RedirectStandardOutput = true,
            UseShellExecute = false
        }) ?? throw new InvalidOperationException("failed to start canonical schema validator");
        process.StandardInput.Write(response);
        process.StandardInput.Close();
        string errors = process.StandardError.ReadToEnd();
        process.WaitForExit();
        if (process.ExitCode != 0)
            throw new InvalidOperationException(
                $"{message} does not match pinned canonical schema: {errors}");
        Console.WriteLine($"PASS: {message} matches pinned canonical schema");
    }

    internal static void ExpectCanonicalReject(string response, string message)
    {
        string? validator = Environment.GetEnvironmentVariable("GAME_INFORMATION_SCHEMA_VALIDATOR");
        string fallback = Path.Combine(Environment.CurrentDirectory, "target", "debug",
            OperatingSystem.IsWindows() ? "game_information_schema_validate.exe" : "game_information_schema_validate");
        validator ??= File.Exists(fallback) ? fallback : null;
        if (validator is null)
            return;
        using var process = Process.Start(new ProcessStartInfo
        {
            FileName = validator, RedirectStandardInput = true, RedirectStandardError = true,
            RedirectStandardOutput = true, UseShellExecute = false
        }) ?? throw new InvalidOperationException("failed to start canonical schema validator");
        process.StandardInput.Write(response);
        process.StandardInput.Close();
        _ = process.StandardError.ReadToEnd();
        process.WaitForExit();
        if (process.ExitCode == 0)
            throw new InvalidOperationException($"{message} unexpectedly passed canonical schema");
        Console.WriteLine($"PASS: {message}");
    }
}
