// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Globalization;
using System.Text;

namespace AiAscension.SessionWindowsBridge;

internal static class GuardianTests
{
    internal static bool TryRunMode(string[] args)
    {
        if (args is ["--tree-probe"])
        {
            using Process descendant = Process.Start(BridgeTests.Self("--long-probe"))!;
            string receipt = Environment.GetEnvironmentVariable("BRIDGE_TEST_DESCENDANT_RECEIPT")!;
            File.WriteAllText(receipt + ".tmp", $"{descendant.Id} {descendant.StartTime.ToUniversalTime().Ticks}");
            File.Move(receipt + ".tmp", receipt);
            Thread.Sleep(30000);
            return true;
        }
        if (args is ["--guardian-no-credential"])
        {
            Program.Main(["--game-executable", Path.GetFullPath("synthetic-never-launched.exe"),
                "--working-directory", Environment.CurrentDirectory, "--bind-address", "127.0.0.1",
                "--port", "12345", "--lease-seconds", "1"]);
            return true;
        }
        if (args is ["--long-probe"])
        {
            Console.Write(new string('x', 1024 * 1024));
            Console.Error.Write(new string('y', 1024 * 1024));
            Thread.Sleep(30000);
            return true;
        }
        if (args.Length != 1 || !args[0].StartsWith("--guardian-", StringComparison.Ordinal))
            return false;
        bool expiry = args[0].Contains("expiry", StringComparison.Ordinal);
        bool stalled = args[0].Contains("stalled", StringComparison.Ordinal);
        using Timer lease = ProcessGuardian.StartLease(expiry ? 3 : 30);
        ProcessGuardian.Run(BridgeTests.Self("--long-probe"), Console.In,
            stalled ? new StalledReceipt() : Console.Out);
        return true;
    }

    internal static void Run()
    {
        DescendantCleanup();
        Scenario("--guardian-normal", "eof");
        Scenario("--guardian-normal", "cancel");
        Scenario("--guardian-normal", "kill");
        Scenario("--guardian-expiry", "expiry");
        Scenario("--guardian-stalled", "eof");
        Scenario("--guardian-stalled", "kill");
        Scenario("--guardian-stalled-expiry", "expiry");
        Console.WriteLine("PASS atomic Job ownership, EOF/cancel, lease, guardian death, stalled receipt");
    }

    private static void DescendantCleanup()
    {
        DirectoryInfo directory = Directory.CreateTempSubdirectory("bridge-job-test-");
        string receipt = Path.Combine(directory.FullName, "synthetic-child.txt");
        using WindowsJob job = WindowsJob.Create();
        ProcessStartInfo options = BridgeTests.Self("--tree-probe");
        options.Environment["BRIDGE_TEST_DESCENDANT_RECEIPT"] = receipt;
        using Process child = DetachedWindowsProcess.Start(options, job);
        Process? descendant = null;
        try
        {
            var deadline = Stopwatch.StartNew();
            while (!File.Exists(receipt) && deadline.ElapsedMilliseconds < 10000) Thread.Sleep(10);
            BridgeTests.Check(File.Exists(receipt), "synthetic descendant receipt bounded");
            string[] fields = File.ReadAllText(receipt).Split(' ');
            descendant = Process.GetProcessById(int.Parse(fields[0], CultureInfo.InvariantCulture));
            _ = descendant.Handle;
            BridgeTests.Check(descendant.StartTime.ToUniversalTime().Ticks
                == long.Parse(fields[1], CultureInfo.InvariantCulture), "pinned synthetic descendant");
            job.Dispose();
            BridgeTests.Check(child.WaitForExit(5000) && descendant.WaitForExit(5000),
                "closing sole Job handle terminates child and descendant");
        }
        finally
        {
            job.Dispose();
            descendant?.Dispose();
            directory.Delete(recursive: true); // Only this test-created synthetic directory.
        }
    }

    internal static void CredentialDeadline()
    {
        ProcessStartInfo options = BridgeTests.Self("--guardian-no-credential");
        options.RedirectStandardInput = true;
        options.RedirectStandardOutput = true;
        using Process guardian = Process.Start(options)!;
        try
        {
            BridgeTests.Check(guardian.WaitForExit(10000) && guardian.ExitCode == 3,
                "lease covers stalled credential read before any spawn");
            BridgeTests.Check(guardian.StandardOutput.ReadToEnd().Length == 0,
                "no receipt when credential was never supplied");
        }
        finally { if (!guardian.HasExited) { guardian.Kill(); guardian.WaitForExit(5000); } }
    }

    private static void Scenario(string mode, string cancellation)
    {
        ProcessStartInfo options = BridgeTests.Self(mode);
        options.RedirectStandardInput = true;
        options.RedirectStandardOutput = true;
        options.RedirectStandardError = true;
        using Process guardian = Process.Start(options)!;
        Process? child = null;
        try
        {
            // The stalled writer exposes identity only to this test side channel, never stdout.
            TextReader receipt = mode.Contains("stalled", StringComparison.Ordinal)
                ? guardian.StandardError : guardian.StandardOutput;
            BridgeTests.Check(ReadLine(receipt) == "STARTED=TRUE", "guardian receipt header");
            int pid = int.Parse(ReadLine(receipt)[4..], CultureInfo.InvariantCulture);
            long ticks = long.Parse(ReadLine(receipt)[12..], CultureInfo.InvariantCulture);
            child = Process.GetProcessById(pid);
            _ = child.Handle;
            BridgeTests.Check(child.StartTime.ToUniversalTime().Ticks == ticks, "pinned test child");
            BridgeTests.Check(!child.HasExited, "child alive before cancellation");
            switch (cancellation)
            {
                case "eof": guardian.StandardInput.Close(); break;
                case "cancel": guardian.StandardInput.WriteLine("CANCEL"); guardian.StandardInput.Flush(); break;
                case "kill": guardian.Kill(); break; // Not entireProcessTree: Job must own cleanup.
            }
            BridgeTests.Check(guardian.WaitForExit(10000), "guardian termination bounded");
            BridgeTests.Check(child.WaitForExit(5000), "Job terminated child without receipt-dependent kill");
            if (cancellation == "expiry") BridgeTests.Check(guardian.ExitCode == 3, "lease expiry status");
            Task<string> remaining = guardian.StandardOutput.ReadToEndAsync();
            BridgeTests.Check(remaining.Wait(1000) && remaining.Result.Length == 0,
                "no secret, arguments, child output, or stalled receipt on stdout");
        }
        finally
        {
            if (!guardian.HasExited) { guardian.Kill(); guardian.WaitForExit(5000); }
            // Only the pinned synthetic test child may be cleaned up if the assertion failed.
            if (child is not null)
            {
                if (!child.HasExited) { child.Kill(true); child.WaitForExit(5000); }
                child.Dispose();
            }
        }
    }

    private static string ReadLine(TextReader reader)
    {
        Task<string?> line = reader.ReadLineAsync();
        BridgeTests.Check(line.Wait(10000), "test receipt read bounded");
        return line.Result ?? throw new InvalidOperationException("missing guardian receipt");
    }

    private sealed class StalledReceipt : TextWriter
    {
        public override Encoding Encoding => Encoding.UTF8;
        public override void Write(string? value)
        {
            Console.Error.Write(value);
            Console.Error.Flush();
            Thread.Sleep(Timeout.Infinite);
        }
    }
}
