// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Read-only enumeration of monitor-compatible modes; never changes the desktop mode.</summary>
internal static class WindowsDisplayModes
{
    internal static IReadOnlyList<(int Width, int Height)> Read(int x, int y, int width, int height)
    {
        var result = new List<(int, int)>();
        if (!OperatingSystem.IsWindows()) return result;
        for (uint index = 0; index < 64; index++)
        {
            var device = new DisplayDevice { Size = Marshal.SizeOf<DisplayDevice>() };
            if (!EnumDisplayDevicesW(null, index, ref device, 0)) break;
            if ((device.Flags & 1) == 0) continue;
            var current = new DeviceMode { Size = (ushort)Marshal.SizeOf<DeviceMode>() };
            if (!EnumDisplaySettingsExW(device.Name, -1, ref current, 0)) continue;
            if (current.X - GetSystemMetrics(76) != x || current.Y - GetSystemMetrics(77) != y
                || current.Width != width || current.Height != height) continue;
            for (int modeIndex = 0; modeIndex < 4096; modeIndex++)
            {
                var mode = new DeviceMode { Size = (ushort)Marshal.SizeOf<DeviceMode>() };
                // No EDS_RAWMODE: Windows filters modes against this monitor's capabilities.
                if (!EnumDisplaySettingsExW(device.Name, modeIndex, ref mode, 0)) break;
                result.Add((mode.Width, mode.Height));
            }
            break;
        }
        return result;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct DisplayDevice
    {
        internal int Size;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] internal string? Name;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] internal string? Description;
        internal uint Flags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] internal string? Id;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] internal string? Key;
    }

    // Fixed Win32 DEVMODEW layout. Only fields used by this read-only adapter are exposed.
    [StructLayout(LayoutKind.Explicit, Size = 220)]
    private struct DeviceMode
    {
        [FieldOffset(68)] internal ushort Size;
        [FieldOffset(76)] internal int X;
        [FieldOffset(80)] internal int Y;
        [FieldOffset(172)] internal int Width;
        [FieldOffset(176)] internal int Height;
    }

    [DllImport("user32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumDisplayDevicesW(string? device, uint index, ref DisplayDevice value, uint flags);

    [DllImport("user32.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumDisplaySettingsExW(string? device, int index, ref DeviceMode value, uint flags);

    [DllImport("user32.dll", ExactSpelling = true)]
    private static extern int GetSystemMetrics(int index);
}
