// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const int RuntimeRequestKindGameFacts = 26;

    private static (int Status, string Response) ProcessGameFactsWork(
        RuntimeContext context, string body)
    {
        if (body.Length == 0)
            return TryAuthorizeRuntimeV2Context(context, out _)
                ? GameFactsNativeBridge.Invoke(_nativeLibrary, 1, string.Empty,
                    string.Empty, context.CorrelationId)
                : (RuntimeRejected, "{\"error_code\":\"game_facts_denied_scope\"}");

        // Rust's canonical codec admits the whole closed request before managed selector reads.
        (int status, string admitted) = GameFactsNativeBridge.Invoke(
            _nativeLibrary, 0, body, string.Empty, context.CorrelationId);
        if (status != RuntimeAccepted)
            return (status, admitted);
        try
        {
            using JsonDocument request = JsonDocument.Parse(admitted);
            JsonElement query = request.RootElement.GetProperty("query");
            JsonElement binding = query.GetProperty("binding");
            if (!TryAuthorizeRuntimeV2Context(context, out _))
                return RefuseGameFacts(context, admitted, "denied_scope");
            if (binding.GetProperty("mode").GetString() != "live")
                return RefuseGameFacts(context, admitted, "missing_capability");
            JsonElement scope = query.GetProperty("scope");
            if (scope.GetProperty("instance_id").GetString() != context.InstanceId
                || scope.GetProperty("locale").GetString() != context.Locale)
                return RefuseGameFacts(context, admitted, "denied_scope");
            string manifest = binding.GetProperty("content_manifest_id").GetString()!;
            ulong epoch = binding.GetProperty("instance_ref").GetProperty("epoch").GetUInt64();
            LiveCardCapturedSnapshot snapshot = ReadRetainedLiveCardSnapshot(
                context.InstanceId, manifest, epoch);
            if (!snapshot.Available)
                return RefuseGameFacts(context, admitted, "stale_snapshot");
            if (!TryReadLiveCardBinding(context, scope.GetProperty("run_id").GetString()!,
                    manifest, scope.GetProperty("authority_epoch").GetUInt64(), snapshot))
                return RefuseGameFacts(context, admitted, "denied_scope");
            LiveCardBindingAssociation association = _liveCardBinding!;
            if (string.IsNullOrEmpty(snapshot.ModeId) || association.CatalogGeneration is null)
                return RefuseGameFacts(context, admitted, "missing_capability");
            if (association.CatalogLocale != context.Locale)
                return RefuseGameFacts(context, admitted, "invalid_binding");

            string catalog = NativeContentCatalogManifestSource.CaptureJson();
            if (Encoding.UTF8.GetByteCount(catalog) > ContentManifestWireContract.MaxMessageBytes)
                return RefuseGameFacts(context, admitted, "missing_capability");
            // The catalog read must not silently replace or advance the selected live capture.
            if (!ReferenceEquals(snapshot,
                    ReadRetainedLiveCardSnapshot(context.InstanceId, manifest, epoch)))
                return RefuseGameFacts(context, admitted, "stale_snapshot");
            using JsonDocument catalogDocument = JsonDocument.Parse(catalog);
            string source = JsonSerializer.Serialize(new Dictionary<string, object?>
            {
                ["catalog"] = catalogDocument.RootElement,
                ["snapshot"] = GameFactsOwnedSnapshot(context, association, snapshot),
                ["refusal"] = null
            });
            return GameFactsNativeBridge.Invoke(
                _nativeLibrary, 2, admitted, source, context.CorrelationId);
        }
        catch (Exception)
        {
            return RefuseGameFacts(context, admitted, "missing_capability");
        }
    }

    private static (int, string) RefuseGameFacts(
        RuntimeContext context, string admitted, string refusal) =>
        GameFactsNativeBridge.Invoke(_nativeLibrary, 2, admitted,
            JsonSerializer.Serialize(new { catalog = (object?)null, snapshot = (object?)null, refusal }),
            context.CorrelationId);
}
