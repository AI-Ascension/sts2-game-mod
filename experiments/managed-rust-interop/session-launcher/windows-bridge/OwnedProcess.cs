// SPDX-License-Identifier: MIT

using System.Diagnostics;
using System.Globalization;

namespace AiAscension.SessionWindowsBridge;

internal static class OwnedProcess
{
    internal static void Stop(string[] args)
    {
        if (!OperatingSystem.IsWindows() || args.Length != 4
            || !int.TryParse(args[1], NumberStyles.None, CultureInfo.InvariantCulture, out int pid)
            || pid <= 0
            || !long.TryParse(args[2], NumberStyles.None, CultureInfo.InvariantCulture, out long ticks)
            || ticks <= 0 || !Path.IsPathFullyQualified(args[3]))
        {
            throw new ArgumentException("invalid owned process identity");
        }
        Process candidate;
        try { candidate = Process.GetProcessById(pid); }
        catch (ArgumentException) { return; } // No such PID: nothing remains to terminate.
        using Process process = candidate;
        // Pin the Windows process object before checking identity: PID reuse cannot redirect kill.
        _ = process.Handle;
        if (process.HasExited)
        {
            return;
        }
        RequireIdentity(ticks, args[3], process.StartTime.ToUniversalTime().Ticks,
            process.MainModule?.FileName);
        process.Kill(entireProcessTree: true);
        if (!process.WaitForExit(5000))
        {
            throw new TimeoutException("owned process termination did not complete");
        }
    }

    internal static void RequireIdentity(long expectedTicks, string expectedExecutable,
        long actualTicks, string? actualExecutable)
    {
        if (expectedTicks <= 0 || actualTicks != expectedTicks || actualExecutable is null
            || !Path.IsPathFullyQualified(expectedExecutable)
            || !Path.IsPathFullyQualified(actualExecutable)
            || !string.Equals(Path.GetFullPath(expectedExecutable), Path.GetFullPath(actualExecutable),
                StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException("owned process identity mismatch");
        }
    }
}
