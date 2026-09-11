// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string SeededRunProtocolVersion = "seeded-run-v1";
    private const string SeededRunArtifact = "sts2-protocol/seeded-run-v1";
    private const string SeededRunSchemaSource = "schemas/seeded-run-v1.schema.json";
    private const string SeededRunGenerator = "hand-authored";
    private const string SeededRunSchemaDigest =
        "5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8";
    private const ulong SeededRunMaximumInteger = 9_007_199_254_740_991;

    private static SeededRunBinding? _seededRunBinding;

    private sealed class SeededRunBinding
    {
        internal SeededRunBinding(RuntimeContext context, ulong leaseEpoch)
        {
            InstanceId = context.InstanceId;
            CallerId = context.CallerId;
            SessionId = context.SessionId;
            LeaseId = context.LeaseId;
            LeaseEpoch = leaseEpoch;
        }

        internal string InstanceId { get; }
        internal string CallerId { get; }
        internal string SessionId { get; }
        internal string LeaseId { get; }
        internal ulong LeaseEpoch { get; }
    }

    private static readonly string[] SeededRunEnvelopeFields =
    {
        "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id",
        "session_id", "lease_id", "lease_epoch", "generation", "kind", "operation_id",
        "requested_seed", "run_mode", "context_digest", "selected_context", "status",
        "canonical_seed", "observation", "effect_witness", "error_code"
    };

    private static readonly string[] SeededRunContextFields =
    {
        "context_id", "game_mode", "character", "ascension", "modifiers", "acts",
        "selection_policy", "profile_baseline", "save_policy", "compatibility", "context_digest"
    };

    private static readonly JsonSerializerOptions SeededRunJsonOptions = new()
    {
        PropertyNameCaseInsensitive = false,
        MaxDepth = 16
    };

    private static (int Status, string Response) ProcessSeededRunWork(RuntimeWork work)
    {
        if (!TryAuthorizeSeededRunContext(work.Context, out string authorizationError))
        {
            return (RuntimeRejected, SeededRunPlainError(authorizationError));
        }

        if (work.Kind == RuntimeRequestKindSeededOperation)
        {
            return ProcessSeededRunReconcile(work.Context, work.Body);
        }

        if (!TryParseSeededStart(work.Body, work.Context, out SeededRunStandardRequest? request,
                out ulong requestGeneration, out string parseError))
        {
            return (400, SeededRunPlainError(parseError));
        }
        SeededRunStandardRequest parsedRequest = request!;

        // A retained operation is replayable even after the host generation advances. This
        // lookup must precede the generation fence so an exact duplicate cannot redispatch.
        if (SeededRunStandardHost.TryGetReceipt(parsedRequest.OperationId, out _))
        {
            SeededRunStandardHostReceipt replay = SeededRunStandardHost.Start(
                parsedRequest, requestGeneration);
            return SeededRunResponse(work.Context, "start_response", replay);
        }

        if (!LiveCombatSource.TryReadCurrentGeneration(out ulong currentGeneration))
        {
            return (RuntimeUnavailable, SeededRunPlainError("runtime_generation_unavailable"));
        }

        if (requestGeneration != currentGeneration)
        {
            SeededRunStandardHostReceipt stale = SeededRunStandardHost.Reject(
                parsedRequest, "stale_generation", requestGeneration);
            (int _, string response) = SeededRunResponse(work.Context, "start_response", stale);
            return (RuntimeRejected, response);
        }

        SeededRunStandardHostReceipt receipt = SeededRunStandardHost.Start(
            parsedRequest, requestGeneration);
        return SeededRunResponse(work.Context, "start_response", receipt);
    }

    private static (int Status, string Response) ProcessSeededRunReconcile(
        RuntimeContext context,
        string operationId)
    {
        if (!SeededRunStandardContract.IsIdentity(operationId))
        {
            return (400, SeededRunPlainError("invalid_operation_id"));
        }

        if (!SeededRunStandardHost.TryGetReceipt(operationId, out _))
        {
            return (404, SeededRunPlainError("operation_not_found"));
        }

        SeededRunStandardHostReceipt receipt = SeededRunStandardHost.Reconcile(operationId);
        return SeededRunResponse(context, "reconcile_response", receipt);
    }

    private static bool TryAuthorizeSeededRunContext(
        RuntimeContext context,
        out string error)
    {
        error = string.Empty;
        if (!ValidRuntimeContext(context)
            || !ulong.TryParse(context.LeaseEpoch, NumberStyles.None, CultureInfo.InvariantCulture,
                out ulong leaseEpoch)
            || leaseEpoch > SeededRunMaximumInteger)
        {
            error = "invalid_seeded_run_context";
            return false;
        }

        SeededRunBinding? binding = _seededRunBinding;
        if (binding is null)
        {
            _seededRunBinding = new SeededRunBinding(context, leaseEpoch);
            return true;
        }

        if (binding.InstanceId != context.InstanceId
            || binding.CallerId != context.CallerId
            || binding.SessionId != context.SessionId
            || binding.LeaseId != context.LeaseId
            || binding.LeaseEpoch != leaseEpoch)
        {
            error = "seeded_run_identity_fence";
            return false;
        }

        return true;
    }

    private static bool TryParseSeededStart(
        string body,
        RuntimeContext context,
        out SeededRunStandardRequest? request,
        out ulong generation,
        out string error)
    {
        request = null;
        generation = 0;
        error = "invalid_seeded_run_envelope";
        try
        {
            using JsonDocument document = JsonDocument.Parse(body,
                new JsonDocumentOptions { MaxDepth = 16 });
            JsonElement root = document.RootElement;
            if (!ExactFields(root, SeededRunEnvelopeFields)
                || StringField(root, "protocol_version") != SeededRunProtocolVersion
                || StringField(root, "schema_digest") != SeededRunSchemaDigest
                || StringField(root, "kind") != "start_request"
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || !BoundedSeededInteger(root.GetProperty("lease_epoch"), out ulong leaseEpoch)
                || leaseEpoch != ParseEpoch(context.LeaseEpoch)
                || !BoundedSeededInteger(root.GetProperty("generation"), out generation)
                || root.GetProperty("status").ValueKind != JsonValueKind.Null
                || root.GetProperty("canonical_seed").ValueKind != JsonValueKind.Null
                || root.GetProperty("observation").ValueKind != JsonValueKind.Null
                || root.GetProperty("effect_witness").ValueKind != JsonValueKind.Null
                || root.GetProperty("error_code").ValueKind != JsonValueKind.Null)
            {
                return false;
            }

            JsonElement provenance = root.GetProperty("provenance");
            if (!ExactFields(provenance, "artifact", "source", "generator")
                || StringField(provenance, "artifact") != SeededRunArtifact
                || StringField(provenance, "source") != SeededRunSchemaSource
                || StringField(provenance, "generator") != SeededRunGenerator)
            {
                return false;
            }

            string? operationId = StringField(root, "operation_id");
            string? requestedSeed = StringField(root, "requested_seed");
            string? runMode = StringField(root, "run_mode");
            string? contextDigest = StringField(root, "context_digest");
            if (!SeededRunStandardContract.IsIdentity(operationId)
                || !SeededRunStandardContract.IsSeed(requestedSeed)
                || runMode is not ("seeded_training" or "seeded_replay" or "diagnostic")
                || !SeededRunStandardContract.IsDigest(contextDigest)
                || !TryParseSelectedContext(root.GetProperty("selected_context"),
                    out SeededRunSelectionContext? selectedContext)
                || selectedContext is null)
            {
                return false;
            }

            request = new SeededRunStandardRequest(
                operationId!, requestedSeed!, runMode!, contextDigest!, selectedContext);
            if (!request.Validate(out error))
            {
                return false;
            }

            error = string.Empty;
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
        catch (InvalidOperationException)
        {
            return false;
        }
    }

    private static bool TryParseSelectedContext(
        JsonElement value,
        out SeededRunSelectionContext? context)
    {
        context = null;
        if (!ExactFields(value, SeededRunContextFields))
        {
            return false;
        }

        JsonElement profile = value.GetProperty("profile_baseline");
        JsonElement compatibility = value.GetProperty("compatibility");
        if (!ExactFields(profile, "kind", "identity", "digest")
            || !ExactFields(compatibility, "game", "mod"))
        {
            return false;
        }

        if (!ExactFields(compatibility.GetProperty("game"), "identity", "digest")
            || !ExactFields(compatibility.GetProperty("mod"), "identity", "digest"))
        {
            return false;
        }

        try
        {
            context = JsonSerializer.Deserialize<SeededRunSelectionContext>(
                value.GetRawText(), SeededRunJsonOptions);
            return context is not null && context.Validate(out _);
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool BoundedSeededInteger(JsonElement value, out ulong number)
    {
        number = 0;
        return value.ValueKind == JsonValueKind.Number
            && value.TryGetUInt64(out number)
            && number <= SeededRunMaximumInteger;
    }

}
