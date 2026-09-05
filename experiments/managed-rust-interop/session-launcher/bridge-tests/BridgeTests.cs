// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Globalization;
using System.Reflection;

namespace AiAscension.SessionWindowsBridge;

internal static class BridgeTests
{
    private static int Main(string[] args)
    {
        if (GuardianTests.TryRunMode(args)) return 0;
        if (args is ["--probe-child"])
        {
            Console.Write(new string('x', 1024 * 1024));
            Console.Error.Write(new string('y', 1024 * 1024));
            Thread.Sleep(3000);
            return 17;
        }
        if (args is ["--launch-child"])
        {
            using Timer lease = ProcessGuardian.StartLease(30);
            ProcessGuardian.Run(Self("--probe-child"), Console.In, Console.Out);
            return 0;
        }

        string executable = Path.GetFullPath("synthetic-host.exe");
        OwnedProcess.RequireIdentity(123, executable, 123, executable);
        Refuses(() => OwnedProcess.RequireIdentity(123, executable, 124, executable));
        Refuses(() => OwnedProcess.RequireIdentity(123, executable, 123, executable + ".other"));
        Refuses(() => OwnedProcess.RequireIdentity(123, executable, 123, null));
        Refuses(() => OwnedProcess.RequireIdentity(0, executable, 0, executable));
        Refuses(() => OwnedProcess.RequireIdentity(123, "relative.exe", 123, executable));
        Check(DetachedWindowsProcess.Quote("") == "\"\"", "empty argument");
        Check(DetachedWindowsProcess.Quote("a\\") == "\"a\\\\\"", "trailing slash");
        Check(DetachedWindowsProcess.Quote("a\"b") == "\"a\\\"b\"", "embedded quote");
        GuardianTests.CredentialDeadline();
        if (OperatingSystem.IsWindows())
        {
            PipeIsolation();
            IdentityCleanup();
            GuardianTests.Run();
            Console.WriteLine("PASS Windows NUL isolation and owned-process cleanup");
        }
        else Console.WriteLine("SKIP Windows process integration: Windows required");
        Console.WriteLine("PASS identity refusal and Windows argument quoting");
        return 0;
    }

    private static void PipeIsolation()
    {
        ProcessStartInfo options = Self("--launch-child");
        options.RedirectStandardOutput = true;
        options.RedirectStandardError = true;
        options.RedirectStandardInput = true;
        using Process launcher = Process.Start(options)!;
        Task<string?> identity = launcher.StandardOutput.ReadLineAsync();
        Check(identity.Wait(5000), "launch identity bounded");
        Check(identity.Result == "STARTED=TRUE", "receipt header");
        string pid = launcher.StandardOutput.ReadLine()!;
        string ticks = launcher.StandardOutput.ReadLine()!;
        using Process child = Process.GetProcessById(int.Parse(pid[4..], CultureInfo.InvariantCulture));
        _ = child.Handle;
        try
        {
            Check(child.StartTime.ToUniversalTime().Ticks == long.Parse(ticks[12..], CultureInfo.InvariantCulture), "child identity");
            Check(child.WaitForExit(5000) && child.ExitCode == 17, "NUL writes do not block");
            Task<string> stdout = launcher.StandardOutput.ReadToEndAsync();
            Task<string> stderr = launcher.StandardError.ReadToEndAsync();
            Check(stdout.Wait(1000) && stdout.Result.Length == 0, "child output not inherited");
            Check(stderr.Wait(1000) && stderr.Result.Length == 0, "child errors not inherited");
            Check(launcher.WaitForExit(1000) && launcher.ExitCode == 0, "guardian exited");
        }
        finally { if (!child.HasExited) { child.Kill(true); child.WaitForExit(5000); } }
    }

    private static void IdentityCleanup()
    {
        using WindowsJob job = WindowsJob.Create();
        using Process child = DetachedWindowsProcess.Start(Self("--probe-child"), job);
        try
        {
            string pid = child.Id.ToString(CultureInfo.InvariantCulture);
            long ticks = child.StartTime.ToUniversalTime().Ticks;
            string executable = child.MainModule!.FileName;
            Refuses(() => OwnedProcess.Stop(["--stop-owned", pid, (ticks + 1).ToString(CultureInfo.InvariantCulture), executable]));
            Check(!child.HasExited, "wrong ticks leaves child alive");
            Refuses(() => OwnedProcess.Stop(["--stop-owned", pid, ticks.ToString(CultureInfo.InvariantCulture), executable + ".other"]));
            Check(!child.HasExited, "wrong executable leaves child alive");
            OwnedProcess.Stop(["--stop-owned", pid, ticks.ToString(CultureInfo.InvariantCulture), executable]);
            Check(child.WaitForExit(1000), "correct identity stops owned child");
        }
        finally { if (!child.HasExited) { child.Kill(true); child.WaitForExit(5000); } }
    }

    internal static ProcessStartInfo Self(string mode)
    {
        string executable = Environment.ProcessPath ?? throw new InvalidOperationException("missing executable");
        var options = new ProcessStartInfo(executable)
        {
            UseShellExecute = false, WorkingDirectory = Environment.CurrentDirectory,
        };
        if (Path.GetFileNameWithoutExtension(executable).Equals("dotnet", StringComparison.OrdinalIgnoreCase))
            options.ArgumentList.Add(Assembly.GetExecutingAssembly().Location);
        options.ArgumentList.Add(mode);
        return options;
    }

    private static void Refuses(Action operation)
    {
        try { operation(); }
        catch (InvalidOperationException) { return; }
        throw new InvalidOperationException("expected identity refusal");
    }
    internal static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
