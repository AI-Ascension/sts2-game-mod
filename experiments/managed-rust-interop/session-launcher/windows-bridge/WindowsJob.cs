// SPDX-License-Identifier: MIT

using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace AiAscension.SessionWindowsBridge;

// Unnamed and non-inheritable: only the guardian owns this kill-on-close handle.
internal sealed class WindowsJob : SafeHandleZeroOrMinusOneIsInvalid
{
    internal WindowsJob() : base(true) { }

    internal static WindowsJob Create()
    {
        WindowsJob job = CreateJobObjectW(IntPtr.Zero, null);
        try
        {
            if (job.IsInvalid) throw new Win32Exception();
            var limits = new ExtendedLimits
            {
                Basic = new BasicLimits { Flags = 0x2000 }, // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            };
            if (!SetInformationJobObject(job, 9, ref limits, Marshal.SizeOf<ExtendedLimits>()))
                throw new Win32Exception();
            return job;
        }
        catch { job.Dispose(); throw; }
    }

    protected override bool ReleaseHandle() => CloseHandle(handle);

    [StructLayout(LayoutKind.Sequential)]
    private struct BasicLimits
    {
        internal long ProcessTime, JobTime;
        internal uint Flags;
        internal UIntPtr MinimumWorkingSet, MaximumWorkingSet;
        internal uint ActiveProcessLimit;
        internal UIntPtr Affinity;
        internal uint PriorityClass, SchedulingClass;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct IoCounters
    {
        internal ulong ReadOperations, WriteOperations, OtherOperations;
        internal ulong ReadBytes, WriteBytes, OtherBytes;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct ExtendedLimits
    {
        internal BasicLimits Basic;
        internal IoCounters Io;
        internal UIntPtr ProcessMemory, JobMemory, PeakProcessMemory, PeakJobMemory;
    }
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern WindowsJob CreateJobObjectW(IntPtr attributes, string? name);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetInformationJobObject(WindowsJob job, int informationClass,
        ref ExtendedLimits information, int informationLength);
    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);
}
