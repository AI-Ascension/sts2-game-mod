// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static void CheckLiveObservationBootstrapVariants()
    {
        Reset();
        string correlation = "corr-bootstrap";
        RuntimeContext context = Context(correlation);
        _ = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2State, context, string.Empty));
        LiveCardCapturedSnapshot snapshot = SyntheticLiveSnapshot();
        ControlledLiveSnapshot = snapshot;
        using JsonDocument scopeDocument = JsonDocument.Parse(SyntheticScope());
        AssociateLiveCardBinding(context, scopeDocument.RootElement, snapshot);

        string knownVariant = MakeBootstrapRequest(correlation, "upgraded");
        (int Status, string Response) known = ProcessLiveObservationBootstrapWork(
            context, knownVariant);
        Check(known.Status == 200
            && JsonDocument.Parse(known.Response).RootElement.GetProperty("kind").GetString()
                == "bootstrap_response",
            "managed bootstrap admits a retained card with its known definition variant");

        (int Status, string Response) mismatch = ProcessLiveObservationBootstrapWork(
            context, MakeBootstrapRequest(correlation, "wrong-variant"));
        Check(mismatch.Status == 404
            && JsonDocument.Parse(mismatch.Response).RootElement.GetProperty("error")
                .GetProperty("code").GetString() == "not_observable",
            "managed bootstrap rejects a mismatched definition variant");

        (int Status, string Response) nullVariant = ProcessLiveObservationBootstrapWork(
            context, MakeBootstrapRequest(correlation, null));
        Check(nullVariant.Status == 404,
            "managed bootstrap does not erase a known variant when selector variant is null");

        (int Status, string Response) stale = ProcessLiveObservationBootstrapWork(
            context, MakeBootstrapRequest(correlation, "upgraded", requestedEpoch: 99));
        Check(stale.Status == 503
            && JsonDocument.Parse(stale.Response).RootElement.GetProperty("error")
                .GetProperty("code").GetString() == "not_observable",
            "managed bootstrap refuses a stale retained snapshot epoch");

        RuntimeContext foreign = new("instance", "foreign", "session", "lease", "1",
            correlation, "en-US");
        (int Status, string Response) foreignResponse = ProcessLiveObservationBootstrapWork(
            foreign, MakeBootstrapRequest(correlation, "upgraded"));
        Check(foreignResponse.Status == 409,
            "managed bootstrap fences a foreign transport owner before source access");
    }

    private static LiveCardCapturedSnapshot SyntheticLiveSnapshot() =>
        new(
            "instance",
            "manifest:1",
            "source:1",
            "run:1",
            "snapshot:1",
            7,
            9,
            new[]
            {
                new LiveCardCapturedCard(
                    "card-instance:1",
                    LiveCardField<string>.Available("base:card:strike"),
                    LiveCardField<string>.Available("manifest:1"),
                    LiveCardField<string>.Available("player:1"),
                    LiveCardField<LiveCardLocation>.Available(
                        new LiveCardLocation(LiveCardZone.Hand, 0)),
                    LiveCardField<ushort>.Available(1),
                    LiveCardField<string>.Available("upgraded"),
                    LiveCardField<string>.NotObserved(),
                    LiveCardField<string>.Available("Strike"),
                    LiveCardField<bool>.Available(true),
                    LiveCardField<int>.Available(1),
                    LiveCardField<int>.NotObserved(),
                    LiveCardField<int>.NotObserved(),
                    LiveCardField<IReadOnlyList<string>>.NotObserved(),
                    LiveCardField<IReadOnlyList<string>>.NotObserved(),
                    LiveCardField<IReadOnlyDictionary<string, string>>.NotObserved())
            },
            true,
            null);

    private static string SyntheticScope() =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["project_id"] = "project:1",
            ["run_id"] = "run:1",
            ["episode_id"] = "episode:1",
            ["agent_id"] = "agent:1",
            ["authority_epoch"] = 1
        });

    private static string MakeBootstrapRequest(
        string correlation,
        string? variant,
        ulong requestedEpoch = 7) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = LiveBootstrapProtocol,
            ["schema_digest"] = LiveBootstrapDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = LiveBootstrapArtifact,
                ["source"] = LiveBootstrapSchemaSource,
                ["generator"] = LiveBootstrapGenerator
            },
            ["correlation_id"] = correlation,
            ["kind"] = "bootstrap_request",
            ["owner_provenance"] = null,
            ["parent_observation"] = null,
            ["error"] = null,
            ["visible_entities"] = null,
            ["scope"] = new Dictionary<string, object?>
            {
                ["instance_id"] = "instance",
                ["run_id"] = "run:1",
                ["content_manifest_id"] = "manifest:1",
                ["locale"] = "en-US",
                ["authority_epoch"] = 1
            },
            ["limits"] = new Dictionary<string, object?>
            {
                ["max_visible_entities"] = 4,
                ["max_item_bytes"] = 65_536,
                ["max_message_bytes"] = 262_144
            },
            ["selector"] = new Dictionary<string, object?>
            {
                ["definition_ref"] = new Dictionary<string, object?>
                {
                    ["content_manifest_id"] = "manifest:1",
                    ["entity_kind"] = "card",
                    ["namespaced_id"] = "base:card:strike",
                    ["variant"] = variant
                },
                ["instance_ref"] = new Dictionary<string, object?>
                {
                    ["instance_id"] = "instance",
                    ["run_id"] = "run:1",
                    ["entity_kind"] = "card",
                    ["entity_id"] = "card-instance:1",
                    ["epoch"] = requestedEpoch
                }
            }
        });
}
