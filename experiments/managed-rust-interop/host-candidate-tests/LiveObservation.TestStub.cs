// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record LiveCardCapturedSnapshot(bool Available, string ContentManifest);

public static partial class ModEntry
{
    private static (int Status, string Response) ProcessLiveObservationBootstrapWork(
        RuntimeContext context,
        string body) =>
        (503, JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["error_code"] = "synthetic_host_unavailable",
            ["correlation_id"] = context.CorrelationId
        }));

    internal static LiveCardCapturedSnapshot ReadLiveCardSnapshot(
        string instanceId,
        string contentManifest) =>
        new(false, string.Empty);

    private static void AssociateLiveCardBinding(
        RuntimeContext context,
        JsonElement scope,
        LiveCardCapturedSnapshot snapshot)
    {
    }
}
