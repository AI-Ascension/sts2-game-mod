// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class ContentManifestWireContract
{
    internal const string ProtocolVersion = "game-information-content-manifest-v1";
    internal const string Artifact = "sts2-protocol/game-information-content-manifest-v1";
    internal const string SchemaSource = "schemas/game-information-content-manifest-v1.schema.json";
    internal const string SchemaDigest =
        "416a39769445e6e462c5d5b5504f29010c255e2116a73094e55c7268e47f2ba6";
    internal const int MaxMessageBytes = 16 * 1024 * 1024;

    internal static bool ValidIdentity(string value) =>
        value.Length is > 0 and <= 256
        && value.All(static character =>
            character is >= 'a' and <= 'z'
                or >= 'A' and <= 'Z'
                or >= '0' and <= '9'
                or '.' or ':' or '/' or '_' or '-');

    internal static bool ValidLocale(string value) => ValidIdentity(value);
}

public static partial class ModEntry
{
    private static (int Status, string Response) ProcessContentManifestWork(RuntimeContext context)
    {
        if (!ContentManifestWireContract.ValidLocale(context.Locale))
        {
            return (400, ContentManifestError(context.CorrelationId,
                "malformed", "invalid_identity"));
        }

        try
        {
            string sourceJson = NativeContentCatalogManifestSource.CaptureJson();
            if (TryProduceContentManifest(
                    sourceJson, context.CorrelationId, out int status, out string response)
                && status == RuntimeAccepted
                && !string.IsNullOrEmpty(response))
            {
                return (status, response);
            }
        }
        catch (InvalidOperationException)
        {
            // Keep host readiness and source details out of the protocol envelope.
        }

        return (503, ContentManifestError(context.CorrelationId,
            "missing_capability", "source_unavailable"));
    }

    private static string ContentManifestError(
        string correlationId,
        string code,
        string reason) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = ContentManifestWireContract.ProtocolVersion,
            ["schema_digest"] = ContentManifestWireContract.SchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = ContentManifestWireContract.Artifact,
                ["source"] = ContentManifestWireContract.SchemaSource,
                ["generator"] = "hand-authored"
            },
            ["correlation_id"] = correlationId,
            ["kind"] = "error_response",
            ["manifest"] = null,
            ["error"] = new Dictionary<string, string>
            {
                ["code"] = code,
                ["reason"] = reason
            }
        });
}
