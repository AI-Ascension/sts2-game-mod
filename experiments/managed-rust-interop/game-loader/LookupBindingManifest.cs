// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Reads the typed content-manifest revision on the queued game thread.</summary>
internal sealed class LookupBindingManifest
{
    private LookupBindingManifest(string contentManifestId)
    {
        ContentManifestId = contentManifestId;
    }

    internal string ContentManifestId { get; }

    internal static LookupBindingManifest Read(string correlationId)
    {
        string sourceJson = NativeContentCatalogManifestSource.CaptureJson();
        if (!ModEntry.TryProduceContentManifest(
                sourceJson, correlationId, out int status, out string response)
            || status != 200
            || string.IsNullOrEmpty(response))
        {
            throw new InvalidOperationException("typed content manifest source unavailable");
        }

        using JsonDocument document = JsonDocument.Parse(response);
        JsonElement root = document.RootElement;
        if (ManifestField(root, "kind") != "content_manifest_response"
            || ManifestField(root, "correlation_id") != correlationId
            || !root.TryGetProperty("manifest", out JsonElement manifest)
            || manifest.ValueKind != JsonValueKind.Object)
        {
            throw new InvalidOperationException("typed content manifest response malformed");
        }
        string? inventoryRevision = ManifestField(manifest, "inventory_revision");
        if (inventoryRevision is null
            || !ContentManifestWireContract.ValidIdentity(inventoryRevision))
        {
            throw new InvalidOperationException("typed content manifest revision unavailable");
        }
        return new LookupBindingManifest(inventoryRevision);
    }

    private static string? ManifestField(JsonElement root, string name) =>
        root.ValueKind == JsonValueKind.Object
            && root.TryGetProperty(name, out JsonElement value)
            && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    internal static bool ValidLocale(string value) =>
        value.Length is >= 2 and <= 35
        && value.Split('-').All(part => part.Length is >= 2 and <= 8
            && part.All(char.IsLetterOrDigit));

    internal static string Digest(string value) =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value))).ToLowerInvariant();
}
