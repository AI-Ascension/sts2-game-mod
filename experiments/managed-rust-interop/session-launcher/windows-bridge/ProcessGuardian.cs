// SPDX-License-Identifier: MIT

using System.Diagnostics;

namespace AiAscension.SessionWindowsBridge;

internal static class ProcessGuardian
{
    internal static Timer StartLease(int seconds)
    {
        ArgumentOutOfRangeException.ThrowIfLessThan(seconds, 1);
        ArgumentOutOfRangeException.ThrowIfGreaterThan(seconds, 3600);
        // Process exit closes the Job even when native creation or receipt I/O is blocked.
        return new Timer(_ => Environment.Exit(3), null, TimeSpan.FromSeconds(seconds),
            Timeout.InfiniteTimeSpan);
    }

    internal static void Run(ProcessStartInfo options, TextReader input, TextWriter output)
    {
        using WindowsJob job = WindowsJob.Create();
        var cancellation = new Thread(() =>
        {
            try { input.Read(); }
            catch (Exception) { Environment.Exit(1); }
            // EOF or any cancellation byte closes every owned descendant, even before receipt.
            Environment.Exit(0);
        }) { IsBackground = true };
        cancellation.Start();
        using Process process = DetachedWindowsProcess.Start(options, job);
        long startTicks = process.StartTime.ToUniversalTime().Ticks;
        output.Write($"STARTED=TRUE\nPID={process.Id}\nSTART_TICKS={startTicks}\n");
        output.Flush();
        process.WaitForExit(); // Independently bounded by the lease and cancellation threads.
        // Disposing the Job also terminates descendants surviving their direct parent.
    }
}
