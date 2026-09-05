// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static bool TryReadPort(out ushort port)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimePortVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            int configuredPort = StandaloneProfileSettings.RuntimePort;
            if (configuredPort <= 0 || configuredPort > ushort.MaxValue)
            {
                port = 0;
                return false;
            }

            port = (ushort)configuredPort;
            return true;
        }
        return ushort.TryParse(value, out port) && port > 0;
    }

    private static bool TryReadBindAddress(out string bindAddress)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimeBindAddressVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            value = StandaloneProfileSettings.RuntimeBindAddress;
        }

        value = value.Trim();
        if (!StandaloneProfileSettings.IsValidRuntimeBindAddress(value))
        {
            bindAddress = string.Empty;
            return false;
        }

        bindAddress = value;
        return true;
    }

}
