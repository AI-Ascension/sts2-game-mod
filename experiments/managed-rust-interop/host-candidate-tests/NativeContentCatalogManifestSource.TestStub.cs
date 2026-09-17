// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class NativeContentCatalogManifestSource
{
    internal static string CaptureJson() =>
        throw new InvalidOperationException("synthetic host does not provide ModelDb");
}

internal sealed class NativeContentCatalogOwnerObservation
{
    internal static NativeContentCatalogOwnerObservation Capture() =>
        throw new InvalidOperationException("synthetic host does not provide ModelDb");
}

public static partial class ModEntry
{
    internal static bool TryProduceContentManifest(
        string input,
        string correlationId,
        out int status,
        out string response)
    {
        status = 503;
        response = string.Empty;
        return false;
    }
}
