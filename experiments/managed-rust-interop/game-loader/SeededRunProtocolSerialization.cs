// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static (int Status, string Response) SeededRunResponse(
        RuntimeContext context,
        string kind,
        SeededRunStandardHostReceipt receipt)
    {
        var payload = new Dictionary<string, object?>
        {
            ["protocol_version"] = SeededRunProtocolVersion,
            ["schema_digest"] = SeededRunSchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = SeededRunArtifact,
                ["source"] = SeededRunSchemaSource,
                ["generator"] = SeededRunGenerator
            },
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            // The envelope generation is the generation carried by the original start request.
            // A settled observation/witness carries the post-start generation separately.
            ["generation"] = receipt.RequestGeneration,
            ["kind"] = kind,
            ["operation_id"] = receipt.OperationId,
            ["requested_seed"] = receipt.RequestedSeed,
            ["run_mode"] = receipt.RunMode,
            ["context_digest"] = receipt.SelectedContext?.ContextDigest,
            ["selected_context"] = receipt.SelectedContext,
            ["status"] = SeededRunStatusText(receipt.Status),
            ["canonical_seed"] = receipt.CanonicalSeed,
            ["observation"] = SeededRunObservationValue(receipt.Observation),
            ["effect_witness"] = SeededRunEffectValue(receipt.EffectWitness),
            ["error_code"] = receipt.ErrorCode
        };
        string response = JsonSerializer.Serialize(payload, SeededRunJsonOptions);
        return (SeededRunStatusCode(receipt.Status), response);
    }

    private static Dictionary<string, object?>? SeededRunObservationValue(
        SeededRunStandardObservation? observation) => observation is null
        ? null
        : new Dictionary<string, object?>
        {
            ["run_started"] = observation.RunStarted,
            ["host_ready"] = observation.HostReady,
            ["generation"] = observation.Generation,
            ["canonical_seed"] = observation.CanonicalSeed,
            ["selected_context_digest"] = observation.SelectedContextDigest,
            ["phase_before"] = observation.PhaseBefore,
            ["phase_after"] = observation.PhaseAfter,
            ["compatibility_identity"] = observation.CompatibilityIdentity
        };

    private static Dictionary<string, object?>? SeededRunEffectValue(
        SeededRunStandardEffectWitness? witness) => witness is null
        ? null
        : new Dictionary<string, object?>
        {
            ["kind"] = witness.Kind,
            ["generation"] = witness.Generation,
            ["canonical_seed"] = witness.CanonicalSeed
        };

    private static string SeededRunStatusText(SeededRunStandardHostStatus status) => status switch
    {
        SeededRunStandardHostStatus.Accepted => "accepted",
        SeededRunStandardHostStatus.Settled => "settled",
        SeededRunStandardHostStatus.Rejected => "rejected",
        SeededRunStandardHostStatus.Unknown => "unknown",
        _ => "unknown"
    };

    private static int SeededRunStatusCode(SeededRunStandardHostStatus status) => status switch
    {
        SeededRunStandardHostStatus.Accepted or SeededRunStandardHostStatus.Settled => RuntimeAccepted,
        SeededRunStandardHostStatus.Rejected => RuntimeRejected,
        SeededRunStandardHostStatus.Unknown => RuntimeUnavailable,
        _ => RuntimeUnavailable
    };

    private static string SeededRunPlainError(string code) => JsonSerializer.Serialize(
        new Dictionary<string, string> { ["error_code"] = code }, SeededRunJsonOptions);
}
