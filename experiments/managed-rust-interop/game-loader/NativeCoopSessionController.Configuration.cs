// SPDX-License-Identifier: MIT

using System;
using System.Globalization;
using RuntimeEnvironment = System.Environment;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeCoopSessionController
{

    private static bool TryReadHostId(out ulong hostId)
    {
        hostId = ParseHostId();
        return hostId != 0;
    }

    private static bool TryReadClientId(out ulong clientId)
    {
        clientId = ParseClientId();
        return clientId != 0;
    }

    private static bool TryReadAutoAdmit(out bool autoAdmit, out string error)
    {
        string? value = RuntimeEnvironment.GetEnvironmentVariable(AutoAdmitVariable)?.Trim();
        if (string.IsNullOrEmpty(value) || value is "0" or "false" or "off")
        {
            autoAdmit = false;
            error = string.Empty;
            return true;
        }
        if (value is "1" or "true" or "on")
        {
            autoAdmit = true;
            error = string.Empty;
            return true;
        }

        autoAdmit = false;
        error = $"{AutoAdmitVariable} must be true or false";
        return false;
    }

    private static ushort ReadPort()
    {
        int value = ReadBoundedInt(PortVariable, DefaultPort, MinPort, MaxPort);
        return checked((ushort)value);
    }

    private static int ReadBoundedInt(string variable, int fallback, int minimum, int maximum)
    {
        string? value = RuntimeEnvironment.GetEnvironmentVariable(variable);
        return int.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out int parsed)
            ? Math.Clamp(parsed, minimum, maximum)
            : fallback;
    }

    private static bool IsLoopbackAddress(string address) =>
        address is "127.0.0.1" or "localhost" or "::1";

    private static ulong ParseHostId()
    {
        string? value = RuntimeEnvironment.GetEnvironmentVariable(HostIdVariable)?.Trim();
        return ulong.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out ulong parsed)
            ? parsed
            : 0;
    }

    private static ulong ParseClientId()
    {
        string? value = RuntimeEnvironment.GetEnvironmentVariable(ClientIdVariable)?.Trim();
        return ulong.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out ulong parsed)
            ? parsed
            : 0;
    }
}
