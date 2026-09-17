// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

// Queue probes exercise dispatch and timeout ownership. They do not claim a live content source;
// this fixture keeps the unrelated content-manifest callback unavailable while compiling the
// shared response envelope.
internal static class NativeContentCatalogManifestSource
{
    internal static string CaptureJson() =>
        throw new InvalidOperationException("queue probe has no content source");
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
