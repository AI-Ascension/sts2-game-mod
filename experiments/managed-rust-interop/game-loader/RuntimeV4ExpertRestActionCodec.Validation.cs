// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    /// <summary>Strictly checks a producer response as a downstream consumer would.</summary>
    internal static bool TryValidateResponse(
        string json,
        RuntimeV4ExpertRestContext context,
        out string error)
    {
        error = string.Empty;
        try
        {
            using JsonDocument document = JsonDocument.Parse(json,
                new JsonDocumentOptions { MaxDepth = 24 });
            JsonElement root = document.RootElement;
            if (!HasExactFields(root,
                    "protocol_version", "schema_digest", "provenance", "profile",
                    "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch",
                    "generation", "state_id", "operation_id", "kind", "action", "status",
                    "observation", "transition", "effect_witness", "error_code"))
            {
                return Fail(out error, "rest response envelope fields are not exact");
            }
            if (StringField(root, "protocol_version")
                != RuntimeV4ExpertRestActionContract.ProtocolVersion
                || StringField(root, "schema_digest")
                != RuntimeV4ExpertRestActionContract.SchemaDigest
                || StringField(root, "profile") != RuntimeV4ExpertRestActionContract.Profile
                || StringField(root, "kind") != "action_response"
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || !UInt64Field(root, "lease_epoch", context.LeaseEpoch)
                || !UInt64Field(root, "generation", out ulong generation)
                || generation > RuntimeV3GameplayContract.MaxGeneration
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "state_id"))
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "operation_id"))
                || !Provenance(root))
            {
                return Fail(out error, "rest response envelope identity is invalid");
            }

            if (!root.TryGetProperty("status", out JsonElement statusValue)
                || statusValue.ValueKind != JsonValueKind.String)
            {
                return Fail(out error, "rest response status is missing");
            }
            string? status = statusValue.GetString();
            if (status is not ("accepted" or "settled" or "rejected" or "unknown" or "cancelled"))
            {
                return Fail(out error, "rest response status is outside the closed set");
            }
            bool actionNull = root.GetProperty("action").ValueKind == JsonValueKind.Null;
            bool observationNull = root.GetProperty("observation").ValueKind == JsonValueKind.Null;
            bool transitionNull = root.GetProperty("transition").ValueKind == JsonValueKind.Null;
            bool witnessNull = root.GetProperty("effect_witness").ValueKind == JsonValueKind.Null;
            bool errorNull = root.GetProperty("error_code").ValueKind == JsonValueKind.Null;

            if (actionNull
                || !TryParseAction(root.GetProperty("action"),
                    out RuntimeV4ExpertRestActionReference? action, out error)
                || action is null)
                return Fail(out error, "rest response action is invalid");

            if (status == "accepted")
            {
                if (actionNull || !observationNull || !transitionNull || !witnessNull || !errorNull)
                    return Fail(out error, "accepted rest response has invalid settlement fields");
                if (!RuntimeV4ExpertRestActionContract.TryValidateAction(action, out error))
                    return false;
                return true;
            }
            if (status is "rejected" or "unknown" or "cancelled")
            {
                if (actionNull || !observationNull || !transitionNull || !witnessNull
                    || errorNull || !RuntimeV4ExpertRestActionContract.IsIdentity(
                        StringField(root, "error_code")))
                    return Fail(out error, "non-settled rest response has invalid fields");
                return RuntimeV4ExpertRestActionContract.TryValidateAction(action, out error);
            }

            if (actionNull || observationNull || transitionNull || !errorNull)
                return Fail(out error, "settled rest response has invalid fields");
            if (!RuntimeV4ExpertRestActionContract.TryValidateAction(action, out error)) return false;
            string stateId = StringField(root, "state_id")!;
            JsonElement observation = root.GetProperty("observation");
            if (!ValidateObservation(observation, stateId, generation, out error)) return false;
            if (!ValidateTransition(root.GetProperty("transition"), generation,
                    root.GetProperty("operation_id").GetString()!, action, observation,
                    out string? transitionOption, out bool transitionWitness, out error))
                return false;
            if (transitionWitness != !witnessNull)
                return Fail(out error, "root and transition witness presence differs");
            if (!witnessNull && !ValidateWitness(root.GetProperty("effect_witness"), generation,
                    root.GetProperty("operation_id").GetString()!, transitionOption!, stateId,
                    out error))
                return false;
            if (!witnessNull && !JsonElementDeepEquals(
                    root.GetProperty("effect_witness"),
                    root.GetProperty("transition").GetProperty("effect_witness")))
                return Fail(out error, "root and transition witnesses differ");
            if (!witnessNull && !ValidateCompletedWitnessBinding(
                    root.GetProperty("effect_witness"), root.GetProperty("transition"), action,
                    transitionOption!, out error))
                return false;
            return true;
        }
        catch (JsonException)
        {
            return Fail(out error, "rest response JSON is invalid");
        }
        catch (InvalidOperationException)
        {
            return Fail(out error, "rest response shape is invalid");
        }
    }

    internal static bool TryParseRequest(
        string json,
        RuntimeV4ExpertRestContext context,
        out RuntimeV4ExpertRestRequest? request,
        out string error)
    {
        request = null;
        error = string.Empty;
        try
        {
            using JsonDocument document = JsonDocument.Parse(json,
                new JsonDocumentOptions { MaxDepth = 24 });
            JsonElement root = document.RootElement;
            if (!HasExactFields(root,
                    "protocol_version", "schema_digest", "provenance", "profile",
                    "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch",
                    "generation", "state_id", "operation_id", "kind", "action", "status",
                    "observation", "transition", "effect_witness", "error_code")
                || StringField(root, "protocol_version")
                    != RuntimeV4ExpertRestActionContract.ProtocolVersion
                || StringField(root, "schema_digest")
                    != RuntimeV4ExpertRestActionContract.SchemaDigest
                || StringField(root, "profile") != RuntimeV4ExpertRestActionContract.Profile
                || StringField(root, "kind") != "action_request"
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || !UInt64Field(root, "lease_epoch", context.LeaseEpoch)
                || !UInt64Field(root, "generation", out ulong generation)
                || generation > RuntimeV3GameplayContract.MaxGeneration
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "state_id"))
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "operation_id"))
                || !Provenance(root)
                || !NullField(root, "status") || !NullField(root, "observation")
                || !NullField(root, "transition") || !NullField(root, "effect_witness")
                || !NullField(root, "error_code"))
            {
                error = "rest action request envelope is invalid";
                return false;
            }
            if (!TryParseAction(root.GetProperty("action"), out RuntimeV4ExpertRestActionReference? action,
                    out error)) return false;
            string stateId = StringField(root, "state_id")!;
            string operationId = StringField(root, "operation_id")!;
            request = new RuntimeV4ExpertRestRequest(
                context,
                new RuntimeV4ExpertRestOperation(context.InstanceId, context.SessionId,
                    context.LeaseId, context.LeaseEpoch, operationId),
                action!, stateId, generation);
            return true;
        }
        catch (JsonException)
        {
            error = "rest action request JSON is invalid";
            return false;
        }
        catch (InvalidOperationException)
        {
            error = "rest action request shape is invalid";
            return false;
        }
    }

}
