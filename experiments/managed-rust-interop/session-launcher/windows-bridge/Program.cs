// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

namespace AiAscension.SessionWindowsBridge;

internal static partial class Program
{
    private const int MinimumCredentialLength = 43;
    private const int MaximumCredentialLength = 256;

    internal static int Main(string[] args)
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
            using Timer lease = ProcessGuardian.StartLease(options.LeaseSeconds);
            string credential = ReadCredential(Console.In);
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

            ProcessGuardian.Run(startInfo, Console.In, Console.Out);
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

    private static string ReadCredential(TextReader input)
    {
        var value = new StringBuilder();
        while (value.Length <= MaximumCredentialLength)
        {
            int next = input.Read();
            if (next is -1 or '\n') return value.ToString().TrimEnd('\r');
            value.Append((char)next);
        }
        return string.Empty;
    }

    [GeneratedRegex("^[A-Za-z0-9_-]+$", RegexOptions.CultureInvariant)]
    private static partial Regex SafeCredentialPattern();

    private sealed class Options
    {
        private Options(string gameExecutable, string workingDirectory, string bindAddress, string port,
            int leaseSeconds)
        {
            GameExecutable = gameExecutable;
            WorkingDirectory = workingDirectory;
            BindAddress = bindAddress;
            Port = port;
            LeaseSeconds = leaseSeconds;
        }

        internal string GameExecutable { get; }
        internal string WorkingDirectory { get; }
        internal string BindAddress { get; }
        internal string Port { get; }
        internal int LeaseSeconds { get; }

        internal static Options Parse(string[] args)
        {
            string? gameExecutable = null;
            string? workingDirectory = null;
            string? bindAddress = null;
            string? port = null;
            int leaseSeconds = 0;
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
                    case "--lease-seconds":
                        if (!int.TryParse(optionValue, NumberStyles.None, CultureInfo.InvariantCulture,
                            out leaseSeconds) || leaseSeconds is < 1 or > 3600)
                            throw new ArgumentException("invalid guardian lease");
                        break;
                    default:
                        throw new ArgumentException("unknown bridge option");
                }
            }

            if (string.IsNullOrWhiteSpace(gameExecutable)
                || string.IsNullOrWhiteSpace(workingDirectory)
                || string.IsNullOrWhiteSpace(bindAddress)
                || string.IsNullOrWhiteSpace(port) || leaseSeconds == 0)
            {
                throw new ArgumentException("bridge options are incomplete");
            }

            return new Options(gameExecutable, workingDirectory, bindAddress, port, leaseSeconds);
        }
    }
}
