// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Globalization;
using System.Reflection;

namespace AiAscension.SessionWindowsBridge;

internal static class BridgeTests
{
    private static int Main(string[] args)
    {
        if (args is ["--probe-child"])
        {
            Console.Write(new string('x', 1024 * 1024));
            Console.Error.Write(new string('y', 1024 * 1024));
            Thread.Sleep(3000);
            return 17;
        }
        if (args is ["--launch-child"])
        {
            using Process child = DetachedWindowsProcess.Start(Self("--probe-child"));
            Console.WriteLine($"{child.Id} {child.StartTime.ToUniversalTime().Ticks}");
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
        if (OperatingSystem.IsWindows())
        {
            PipeIsolation();
            IdentityCleanup();
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
        using Process launcher = Process.Start(options)!;
        Task<string?> identity = launcher.StandardOutput.ReadLineAsync();
        Check(identity.Wait(5000), "launch identity bounded");
        string[] fields = (identity.Result ?? throw new InvalidOperationException("missing identity")).Split(' ');
        using Process child = Process.GetProcessById(int.Parse(fields[0], CultureInfo.InvariantCulture));
        _ = child.Handle;
        try
        {
            Check(child.StartTime.ToUniversalTime().Ticks == long.Parse(fields[1], CultureInfo.InvariantCulture), "child identity");
            Check(launcher.StandardOutput.ReadToEndAsync().Wait(1000), "stdout EOF before child exit");
            Check(launcher.StandardError.ReadToEndAsync().Wait(1000), "stderr EOF before child exit");
            Check(!child.HasExited, "pipe EOF while child alive");
            Check(launcher.WaitForExit(1000) && launcher.ExitCode == 0, "launch helper exited");
            Check(child.WaitForExit(5000) && child.ExitCode == 17, "NUL writes do not block");
        }
        finally { if (!child.HasExited) { child.Kill(true); child.WaitForExit(5000); } }
    }

    private static void IdentityCleanup()
    {
        using Process child = DetachedWindowsProcess.Start(Self("--probe-child"));
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

    private static ProcessStartInfo Self(string mode)
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
    private static void Check(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
