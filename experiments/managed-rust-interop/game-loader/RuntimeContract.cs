// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string RuntimeArtifact = "sts2-protocol/runtime-v1";
    private const string RuntimeSchemaSource = "schemas/runtime-v1.schema.json";
    private const string RuntimeGenerator = "hand-authored";
    private const string RuntimeSchemaDigest = "a76086d7a68668fd4cff53999369d2b450b0d6623827393882f458f2aa1f93eb";

    private static (int Status, string Response) ProcessRuntimeWork(RuntimeWork work)
    {
        if (work.Kind == RuntimeRequestKindState)
        {
            return (RuntimeAccepted, RuntimeStateResponse(work.Context));
        }
        if (work.Kind != RuntimeRequestKindAction)
        {
            return (400, RuntimeError(work.Context, work.Kind, "unknown_request_kind"));
        }
        return ProcessRuntimeAction(work.Context, work.Body);
    }

    private static (int Status, string Response) ProcessRuntimeAction(RuntimeContext context, string body)
    {
        using JsonDocument document = JsonDocument.Parse(body, new JsonDocumentOptions { MaxDepth = 8 });
        JsonElement root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object
            || StringField(root, "protocol_version") != "runtime-v1"
            || StringField(root, "schema_digest") != RuntimeSchemaDigest
            || StringField(root, "kind") != "action_request"
            || StringField(root, "instance_id") != context.InstanceId
            || StringField(root, "session_id") != context.SessionId
            || StringField(root, "lease_id") != context.LeaseId
            || StringField(root, "correlation_id") != context.CorrelationId
            || StringField(root, "provenance.artifact") != RuntimeArtifact
            || StringField(root, "provenance.source") != RuntimeSchemaSource
            || StringField(root, "provenance.generator") != RuntimeGenerator)
        {
            return (400, RuntimeError(context, RuntimeRequestKindAction, "invalid_runtime_envelope"));
        }
        if (!root.TryGetProperty("generation", out JsonElement generationElement)
            || !generationElement.TryGetUInt64(out ulong expectedGeneration))
        {
            return (400, RuntimeError(context, RuntimeRequestKindAction, "invalid_generation"));
        }
        if (expectedGeneration != _runtimeGeneration)
        {
            return (RuntimeRejected, RuntimeActionResponse(context, expectedGeneration, "rejected", "sts2.game-mod/stale_generation", false));
        }
        if (!root.TryGetProperty("action", out JsonElement action)
            || action.ValueKind != JsonValueKind.Object
            || StringField(action, "action_id") != "show_runtime_probe")
        {
            return (400, RuntimeError(context, RuntimeRequestKindAction, "unsupported_action"));
        }
        if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
        {
            return (RuntimeUnavailable, RuntimeError(context, RuntimeRequestKindAction, "host_not_ready"));
        }
        AddStatusOverlay(tree, ExpectedAbiVersion, ExpectedCheckedAddResult, true);
        if (tree.Root.GetNodeOrNull<CanvasLayer>(StatusNodeName) == null)
        {
            return (RuntimeUnavailable, RuntimeError(context, RuntimeRequestKindAction, "effect_not_observed"));
        }
        _runtimeGeneration++;
        _runtimeActionCount++;
        return (RuntimeAccepted, RuntimeActionResponse(context, _runtimeGeneration, "accepted", null, true));
    }

    private static string RuntimeStateResponse(RuntimeContext context)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = "runtime-v1",
            ["schema_digest"] = RuntimeSchemaDigest,
            ["provenance"] = RuntimeProvenance(),
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            ["generation"] = _runtimeGeneration,
            ["kind"] = "state_response",
            ["observation"] = RuntimeObservation(),
            ["action"] = null,
            ["status"] = null,
            ["error_code"] = null,
            ["effect_witness"] = null
        });
    }

    private static string RuntimeActionResponse(RuntimeContext context, ulong generation, string status, string? errorCode, bool witnessed)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = "runtime-v1",
            ["schema_digest"] = RuntimeSchemaDigest,
            ["provenance"] = RuntimeProvenance(),
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            ["generation"] = _runtimeGeneration,
            ["kind"] = "action_response",
            ["observation"] = RuntimeObservation(),
            ["action"] = new Dictionary<string, object?> { ["action_id"] = "show_runtime_probe" },
            ["status"] = status,
            ["error_code"] = errorCode,
            ["effect_witness"] = witnessed
                ? new Dictionary<string, object?> { ["kind"] = "status_overlay_visible", ["generation"] = generation }
                : null
        });
    }

    private static Dictionary<string, object?> RuntimeObservation()
    {
        SceneTree? tree = Engine.GetMainLoop() as SceneTree;
        bool hostReady = tree?.Root != null;
        bool overlayVisible = hostReady && tree!.Root.GetNodeOrNull<CanvasLayer>(StatusNodeName) != null;
        return new Dictionary<string, object?>
        {
            ["host_ready"] = hostReady,
            ["overlay_visible"] = overlayVisible,
            ["screen"] = hostReady ? "host" : "unavailable",
            ["action_count"] = _runtimeActionCount
        };
    }

    private static string RuntimeError(RuntimeContext context, uint kind, string code)
    {
        return kind == RuntimeRequestKindAction
            ? RuntimeActionResponse(context, _runtimeGeneration, "rejected", code, false)
            : JsonSerializer.Serialize(new Dictionary<string, object?>
            {
                ["protocol_version"] = "runtime-v1",
                ["schema_digest"] = RuntimeSchemaDigest,
                ["provenance"] = RuntimeProvenance(),
                ["correlation_id"] = context.CorrelationId,
                ["instance_id"] = context.InstanceId,
                ["session_id"] = context.SessionId,
                ["lease_id"] = context.LeaseId,
                ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
                ["generation"] = _runtimeGeneration,
                ["kind"] = "state_response",
                ["observation"] = RuntimeObservation(),
                ["action"] = null,
                ["status"] = null,
                ["error_code"] = code,
                ["effect_witness"] = null
            });
    }

    private static Dictionary<string, string> RuntimeProvenance() => new()
    {
        ["artifact"] = RuntimeArtifact,
        ["source"] = RuntimeSchemaSource,
        ["generator"] = RuntimeGenerator
    };

    private static string? StringField(JsonElement root, string path)
    {
        JsonElement value = root;
        foreach (string segment in path.Split('.'))
        {
            if (value.ValueKind != JsonValueKind.Object || !value.TryGetProperty(segment, out value))
            {
                return null;
            }
        }
        return value.ValueKind == JsonValueKind.String ? value.GetString() : null;
    }

    private static ulong ParseEpoch(string value) => ulong.TryParse(value, out ulong epoch) ? epoch : 0;
}
