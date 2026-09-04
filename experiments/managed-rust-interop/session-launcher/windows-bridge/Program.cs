// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Text.RegularExpressions;

namespace AiAscension.SessionWindowsBridge;

internal static partial class Program
{
    private const int MinimumCredentialLength = 43;
    private const int MaximumCredentialLength = 256;

    private static int Main(string[] args)
    {
        try
        {
            if (args.Length > 0 && args[0] == "--stop-owned")
            {
                OwnedProcess.Stop(args);
                Console.WriteLine("STOPPED=TRUE");
                return 0;
            }
            Options options = Options.Parse(args);
            string credential = Console.ReadLine() ?? string.Empty;
            if (!CredentialIsSafe(credential))
            {
                Console.Error.WriteLine("runtime session credential is missing or unsafe");
                return 2;
            }

            var startInfo = new ProcessStartInfo
            {
                FileName = options.GameExecutable,
                WorkingDirectory = options.WorkingDirectory,
                UseShellExecute = false,
                CreateNoWindow = true,
            };
            startInfo.ArgumentList.Add("--headless");
            startInfo.ArgumentList.Add("--audio-driver");
            startInfo.ArgumentList.Add("Dummy");
            startInfo.Environment["STS2_RUNTIME_TOKEN"] = credential;
            startInfo.Environment["STS2_RUNTIME_BIND_ADDRESS"] = options.BindAddress;
            startInfo.Environment["STS2_RUNTIME_PORT"] = options.Port;
            startInfo.Environment["STS2_RUNTIME_SESSION"] = "1";

            using Process process = DetachedWindowsProcess.Start(startInfo);
            int parsedPid = process.Id;
            if (parsedPid <= 0)
            {
                throw new InvalidOperationException("Windows process handoff returned no game PID");
            }
            try
            {
                long startTicks = process.StartTime.ToUniversalTime().Ticks;
                Console.Write($"STARTED=TRUE\nPID={parsedPid}\nSTART_TICKS={startTicks}\n");
                Console.Out.Flush();
            }
            catch
            {
                if (!process.HasExited)
                {
                    process.Kill(entireProcessTree: true);
                    if (!process.WaitForExit(5000)) throw new TimeoutException("handoff cleanup failed");
                }
                throw;
            }
            return 0;
        }
        catch (Exception)
        {
            Console.Error.WriteLine("runtime session process operation failed");
            return 1;
        }
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
