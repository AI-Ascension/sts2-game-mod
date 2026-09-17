// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeContentCatalogManifestSource
{
    internal static string? ControlledJson { get; set; }

    internal static string CaptureJson() =>
        ControlledJson
        ?? throw new InvalidOperationException("synthetic host does not provide ModelDb");
}

internal sealed class NativeContentCatalogOwnerObservation
{
    internal static NativeContentCatalogOwnerObservation Capture() =>
        throw new InvalidOperationException("synthetic host does not provide ModelDb");
}

public static partial class ModEntry
{
    internal static string? ControlledManifestResponse { get; set; }

    internal static bool TryProduceContentManifest(
        string input,
        string correlationId,
        out int status,
        out string response)
    {
        status = ControlledManifestResponse is null ? 503 : 200;
        response = ControlledManifestResponse ?? string.Empty;
        return ControlledManifestResponse is not null;
    }
}
