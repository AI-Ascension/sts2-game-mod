// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.IO;
using System.Text.RegularExpressions;

namespace AiAscension.SessionWindowsBridge;

internal static partial class Program
{
    private const int MinimumCredentialLength = 43;
    private const int MaximumCredentialLength = 256;
    private static readonly char[] LineSeparators = new[] { '\r', '\n' };

    private static int Main(string[] args)
    {
        try
        {
            Options options = Options.Parse(args);
            string credential = Console.ReadLine() ?? string.Empty;
            if (!CredentialIsSafe(credential))
            {
                Console.Error.WriteLine("runtime session credential is missing or unsafe");
                return 2;
            }

            string powerShell = Path.Combine(
                Environment.SystemDirectory,
                "WindowsPowerShell",
                "v1.0",
                "powershell.exe");
            var startInfo = new ProcessStartInfo
            {
                FileName = powerShell,
                UseShellExecute = false,
                CreateNoWindow = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
            };
            startInfo.Environment["STS2_RUNTIME_TOKEN"] = credential;
            startInfo.Environment["STS2_RUNTIME_BIND_ADDRESS"] = options.BindAddress;
            startInfo.Environment["STS2_RUNTIME_PORT"] = options.Port;
            startInfo.Environment["STS2_RUNTIME_SESSION"] = "1";
            startInfo.ArgumentList.Add("-NoProfile");
            startInfo.ArgumentList.Add("-NonInteractive");
            startInfo.ArgumentList.Add("-Command");
            startInfo.ArgumentList.Add(
                $"$game = Start-Process -FilePath {QuotePowerShell(options.GameExecutable)} "
                + $"-WorkingDirectory {QuotePowerShell(options.WorkingDirectory)} -PassThru; "
                + "Write-Output ('PID=' + $game.Id)");

            using Process process = Process.Start(startInfo)
                ?? throw new InvalidOperationException("game process did not start");
            Task<string> outputTask = process.StandardOutput.ReadToEndAsync();
            Task<string> errorTask = process.StandardError.ReadToEndAsync();
            process.WaitForExit();
            _ = errorTask.GetAwaiter().GetResult();
            string processOutput = outputTask.GetAwaiter().GetResult();
            if (process.ExitCode != 0)
            {
                throw new InvalidOperationException("Windows process handoff failed");
            }
            string? gamePid = processOutput
                .Split(LineSeparators, StringSplitOptions.RemoveEmptyEntries)
                .Select(line => line.Trim())
                .FirstOrDefault(line => line.StartsWith("PID=", StringComparison.Ordinal));
            if (gamePid is null || !int.TryParse(gamePid[4..], out int parsedPid) || parsedPid <= 0)
            {
                throw new InvalidOperationException("Windows process handoff returned no game PID");
            }
            Console.WriteLine("STARTED=TRUE");
            Console.WriteLine($"PID={parsedPid}");
            return 0;
        }
        catch (Exception)
        {
            Console.Error.WriteLine("runtime session game launch failed");
            return 1;
        }
    }

    private static string QuotePowerShell(string value)
    {
        return $"'{value.Replace("'", "''", StringComparison.Ordinal)}'";
    }

    private static bool CredentialIsSafe(string value)
    {
        return value.Length is >= MinimumCredentialLength and <= MaximumCredentialLength
            && SafeCredentialPattern().IsMatch(value);
    }

    [GeneratedRegex("^[A-Za-z0-9_-]+$", RegexOptions.CultureInvariant)]
    private static partial Regex SafeCredentialPattern();

    private sealed class Options
    {
        private Options(string gameExecutable, string workingDirectory, string bindAddress, string port)
        {
            GameExecutable = gameExecutable;
            WorkingDirectory = workingDirectory;
            BindAddress = bindAddress;
            Port = port;
        }

        internal string GameExecutable { get; }
        internal string WorkingDirectory { get; }
        internal string BindAddress { get; }
        internal string Port { get; }

        internal static Options Parse(string[] args)
        {
            string? gameExecutable = null;
            string? workingDirectory = null;
            string? bindAddress = null;
            string? port = null;
            for (int index = 0; index < args.Length; index++)
            {
                string value = args[index];
                if (index + 1 >= args.Length)
                {
                    throw new ArgumentException("bridge option is missing a value");
                }

                string optionValue = args[++index];
                switch (value)
                {
                    case "--game-executable":
                        gameExecutable = optionValue;
                        break;
                    case "--working-directory":
                        workingDirectory = optionValue;
                        break;
                    case "--bind-address":
                        bindAddress = optionValue;
                        break;
                    case "--port":
                        port = optionValue;
                        break;
                    default:
                        throw new ArgumentException("unknown bridge option");
                }
            }

            if (string.IsNullOrWhiteSpace(gameExecutable)
                || string.IsNullOrWhiteSpace(workingDirectory)
                || string.IsNullOrWhiteSpace(bindAddress)
                || string.IsNullOrWhiteSpace(port))
            {
                throw new ArgumentException("bridge options are incomplete");
            }

            return new Options(gameExecutable, workingDirectory, bindAddress, port);
        }
    }
}
