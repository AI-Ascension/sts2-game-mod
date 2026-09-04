// SPDX-License-Identifier: MIT

using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace AiAscension.SessionWindowsBridge;

// Only NUL handles may cross this boundary, never the bridge's credential or response pipes.
internal static class DetachedWindowsProcess
{
    internal static Process Start(ProcessStartInfo options)
    {
        if (!OperatingSystem.IsWindows() || !Path.IsPathFullyQualified(options.FileName))
        {
            throw new PlatformNotSupportedException("Windows executable required");
        }
        var security = new SecurityAttributes
        {
            Length = Marshal.SizeOf<SecurityAttributes>(), InheritHandle = true,
        };
        using SafeFileHandle nul = CreateFileW("NUL", 0xc0000000, 3, ref security, 3, 0, IntPtr.Zero);
        if (nul.IsInvalid) throw new Win32Exception();
        IntPtr size = IntPtr.Zero;
        InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref size);
        if (size == IntPtr.Zero) throw new Win32Exception();
        IntPtr attributes = Marshal.AllocHGlobal(size);
        IntPtr handles = Marshal.AllocHGlobal(IntPtr.Size);
        IntPtr environment = IntPtr.Zero;
        bool initialized = false;
        ProcessInformation child = default;
        try
        {
            if (!InitializeProcThreadAttributeList(attributes, 1, 0, ref size))
                throw new Win32Exception();
            initialized = true;
            Marshal.WriteIntPtr(handles, nul.DangerousGetHandle());
            // PROC_THREAD_ATTRIBUTE_HANDLE_LIST: do not inherit arbitrary parent handles.
            if (!UpdateProcThreadAttribute(attributes, 0, (IntPtr)0x20002, handles,
                (IntPtr)IntPtr.Size, IntPtr.Zero, IntPtr.Zero)) throw new Win32Exception();
            var startup = new StartupInfoEx
            {
                Startup = new StartupInfo
                {
                    Size = Marshal.SizeOf<StartupInfoEx>(), Flags = 0x100,
                    StandardInput = nul.DangerousGetHandle(),
                    StandardOutput = nul.DangerousGetHandle(),
                    StandardError = nul.DangerousGetHandle(),
                },
                AttributeList = attributes,
            };
            string block = string.Join('\0', options.Environment
                .Where(pair => pair.Value is not null)
                .OrderBy(pair => pair.Key, StringComparer.OrdinalIgnoreCase)
                .Select(pair => $"{pair.Key}={pair.Value}")) + "\0\0";
            environment = Marshal.StringToHGlobalUni(block);
            var command = new StringBuilder(Quote(options.FileName));
            foreach (string argument in options.ArgumentList) command.Append(' ').Append(Quote(argument));
            // CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT.
            char[] commandBuffer = (command.ToString() + '\0').ToCharArray();
            if (!CreateProcessW(options.FileName, commandBuffer, IntPtr.Zero, IntPtr.Zero, true,
                0x08080400, environment, options.WorkingDirectory, ref startup, out child))
                throw new Win32Exception();
            try
            {
                Process process = Process.GetProcessById(child.ProcessId);
                try { _ = process.Handle; return process; }
                catch { process.Dispose(); throw; }
            }
            catch
            {
                if (!TerminateProcess(child.Process, 1)) throw new Win32Exception();
                throw;
            }
        }
        finally
        {
            if (child.Thread != IntPtr.Zero) CloseHandle(child.Thread);
            if (child.Process != IntPtr.Zero) CloseHandle(child.Process);
            if (environment != IntPtr.Zero) Marshal.FreeHGlobal(environment);
            Marshal.FreeHGlobal(handles);
            if (initialized) DeleteProcThreadAttributeList(attributes);
            Marshal.FreeHGlobal(attributes);
        }
    }

    // Windows CommandLineToArgvW/CRT escaping; never interpreted by a shell.
    internal static string Quote(string value)
    {
        var result = new StringBuilder("\"");
        int slashes = 0;
        foreach (char character in value)
        {
            if (character == '\\') { slashes++; continue; }
            result.Append('\\', character == '"' ? slashes * 2 + 1 : slashes);
            result.Append(character);
            slashes = 0;
        }
        return result.Append('\\', slashes * 2).Append('"').ToString();
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SecurityAttributes { internal int Length; internal IntPtr Descriptor;
        [MarshalAs(UnmanagedType.Bool)] internal bool InheritHandle; }
    [StructLayout(LayoutKind.Sequential)]
    private struct StartupInfo
    {
        internal int Size;
        internal IntPtr Reserved, Desktop, Title;
        internal int X, Y, XSize, YSize, XCountChars, YCountChars, FillAttribute, Flags;
        internal short WindowDisplay, ReservedSize;
        internal IntPtr ReservedPointer, StandardInput, StandardOutput, StandardError;
    }
    [StructLayout(LayoutKind.Sequential)]
    private struct StartupInfoEx { internal StartupInfo Startup; internal IntPtr AttributeList; }
    [StructLayout(LayoutKind.Sequential)]
    private struct ProcessInformation { internal IntPtr Process, Thread; internal int ProcessId, ThreadId; }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern SafeFileHandle CreateFileW(string name, uint access, uint sharing,
        ref SecurityAttributes security, uint disposition, uint flags, IntPtr template);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool InitializeProcThreadAttributeList(IntPtr list, int count,
        int flags, ref IntPtr size);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool UpdateProcThreadAttribute(IntPtr list, uint flags, IntPtr attribute,
        IntPtr value, IntPtr size, IntPtr previous, IntPtr returned);
    [DllImport("kernel32.dll")]
    private static extern void DeleteProcThreadAttributeList(IntPtr list);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateProcessW(string application, [In, Out] char[] command,
        IntPtr processAttributes, IntPtr threadAttributes, [MarshalAs(UnmanagedType.Bool)] bool inherit,
        uint flags, IntPtr environment, string directory, ref StartupInfoEx startup,
        out ProcessInformation information);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool TerminateProcess(IntPtr process, uint exitCode);
    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);
}
