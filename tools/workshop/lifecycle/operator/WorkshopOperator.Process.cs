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
    private static (int? ExitCode, bool TimedOut) RunProcess(
        string[] command,
        Dictionary<string, string>? environment,
        int timeout,
        FileStream log)
    {
        if (command.Length == 0)
            throw new OperatorException("operation command is empty");
        ProcessStartInfo start = new()
        {
            FileName = command[0],
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        foreach (string argument in command.Skip(1))
            start.ArgumentList.Add(argument);
        if (environment is not null)
        {
            foreach ((string name, string value) in environment)
                start.Environment[name] = value;
        }
        using Process process = new() { StartInfo = start };
        if (!process.Start())
            throw new InvalidOperationException("the child process did not start");
        Task output = process.StandardOutput.BaseStream.CopyToAsync(log);
        Task error = process.StandardError.BaseStream.CopyToAsync(log);
        process.StandardInput.Close();
        Stopwatch stopwatch = Stopwatch.StartNew();
        bool timedOut = false;
        while (!process.WaitForExit(100))
        {
            if (stopwatch.Elapsed >= TimeSpan.FromSeconds(timeout))
            {
                timedOut = true;
                try
                {
                    process.Kill(entireProcessTree: true);
                }
                catch
                {
                    // The durable journal is classified unknown regardless of kill outcome.
                }
                process.WaitForExit();
                break;
            }
        }
        process.WaitForExit();
        Task.WaitAll(output, error);
        return (process.ExitCode, timedOut);
    }

    private static FileStream CreatePrivateFile(string path)
    {
        FileStream stream = new(
            path,
            FileMode.CreateNew,
            FileAccess.Write,
            FileShare.Read,
            4096,
            FileOptions.WriteThrough);
        SetPrivateMode(path);
        return stream;
    }

    private static void WriteJsonExclusive(string path, object value)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path) ?? ".");
        byte[] bytes = Encoding.UTF8.GetBytes(Serialize(value) + "\n");
        using FileStream stream = new(
            path,
            FileMode.CreateNew,
            FileAccess.Write,
            FileShare.None,
            4096,
            FileOptions.WriteThrough);
        stream.Write(bytes);
        stream.Flush(flushToDisk: true);
        SetPrivateMode(path);
    }

    private static void WriteJsonAtomic(string path, object value)
    {
        string directory = Path.GetDirectoryName(path) ?? ".";
        Directory.CreateDirectory(directory);
        string temporary = Path.Combine(
            directory,
            $".{Path.GetFileName(path)}.{Environment.ProcessId}.{Guid.NewGuid():N}.tmp");
        try
        {
            WriteJsonExclusive(temporary, value);
            File.Move(temporary, path, overwrite: true);
            SetPrivateMode(path);
        }
        finally
        {
            if (File.Exists(temporary))
            {
                try { File.Delete(temporary); } catch { /* Preserve the durable journal. */ }
            }
        }
    }

    private static string UtcNow() => DateTimeOffset.UtcNow.ToString("O");

    private static void SetPrivateMode(string path)
    {
        if (!OperatingSystem.IsWindows())
            File.SetUnixFileMode(path, UnixFileMode.UserRead | UnixFileMode.UserWrite);
    }
}
