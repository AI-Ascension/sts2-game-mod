// SPDX-License-Identifier: MIT

using System.Globalization;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private static string NativeCheckpointId(ulong checksumOrdinal) =>
        $"checkpoint:native-{checksumOrdinal.ToString(CultureInfo.InvariantCulture)}";
}
