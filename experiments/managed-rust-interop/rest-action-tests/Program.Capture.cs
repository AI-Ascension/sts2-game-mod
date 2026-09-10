// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.RestActionTests;

internal static partial class Program
{
    private static readonly JsonSerializerOptions FixtureJsonOptions = new() { WriteIndented = true };

    private static void EmitProducerFixtures(string directory)
    {
        if (string.IsNullOrWhiteSpace(directory))
            throw new ArgumentException("capture output directory is required", nameof(directory));

        Directory.CreateDirectory(directory);
        WriteFixture(directory, "smith-selection-lifecycle.json", "producer/smith-selection-lifecycle-v1",
            "rest_option -> selection_requested -> selection_progressed -> selection_completed",
            SmithMessages());
        WriteFixture(directory, "mend-selection-lifecycle.json", "producer/mend-selection-lifecycle-v1",
            "rest_option -> selection_requested -> selection_completed",
            MendMessages());
    }

    private static List<object> SmithMessages()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:producer:smith");
        var host = new SmithHost(captureIdentity: true);
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());
        RuntimeV4ExpertRestHostProjection initial = host.ObserveRest();
        RuntimeV4ExpertRestRequest parent = RequestFromProjection(context, initial,
            "rest-op:producer:smith:parent", initial.LegalActions[0]);
        var messages = new List<object>
        {
            CaptureMessage(support, context, "option-request", parent)
        };

        RuntimeV4ExpertRestHostProjection firstSurface = host.ObserveRest();
        RuntimeV4ExpertRestActionReference firstAction = FindAction(firstSurface,
            "select_card", "card:1");
        RuntimeV4ExpertRestRequest first = RequestFromProjection(context, firstSurface,
            "rest-op:producer:smith:first", firstAction);
        messages.Add(CaptureMessage(support, context, "first-choice", first));

        RuntimeV4ExpertRestHostProjection secondSurface = host.ObserveRest();
        RuntimeV4ExpertRestActionReference secondAction = FindAction(secondSurface,
            "select_card", "card:2");
        RuntimeV4ExpertRestRequest second = RequestFromProjection(context, secondSurface,
            "rest-op:producer:smith:second", secondAction);
        messages.Add(CaptureMessage(support, context, "second-choice", second));

        RuntimeV4ExpertRestHostProjection confirmSurface = host.ObserveRest();
        RuntimeV4ExpertRestActionReference confirmAction = FindAction(confirmSurface,
            "confirm_selection", null);
        RuntimeV4ExpertRestRequest confirmation = RequestFromProjection(context, confirmSurface,
            "rest-op:producer:smith:confirm", confirmAction);
        messages.Add(CaptureMessage(support, context, "confirm", confirmation));
        return messages;
    }

    private static List<object> MendMessages()
    {
        RuntimeV4ExpertRestContext context = new("instance:1", "session:1", "lease:1", 4,
            "corr:rest:producer:mend");
        var host = new MendHost(captureIdentity: true);
        RuntimeV4ExpertRestActionSupport support =
            RuntimeV4ExpertRestActionSupport.WithHost(host, work => work());
        RuntimeV4ExpertRestHostProjection initial = host.ObserveRest();
        RuntimeV4ExpertRestRequest parent = RequestFromProjection(context, initial,
            "rest-op:producer:mend:parent", initial.LegalActions[0]);
        var messages = new List<object>
        {
            CaptureMessage(support, context, "option-request", parent)
        };

        RuntimeV4ExpertRestHostProjection targetSurface = host.ObserveRest();
        RuntimeV4ExpertRestActionReference targetAction = FindAction(targetSurface,
            "select_player", "player:local");
        RuntimeV4ExpertRestRequest target = RequestFromProjection(context, targetSurface,
            "rest-op:producer:mend:target", targetAction);
        messages.Add(CaptureMessage(support, context, "player-choice", target));
        return messages;
    }

    private static RuntimeV4ExpertRestActionReference FindAction(
        RuntimeV4ExpertRestHostProjection projection, string kind, string? choice)
    {
        foreach (RuntimeV4ExpertRestActionReference action in projection.LegalActions)
        {
            if (action.Action.Kind != kind) continue;
            if (choice is null || action.Action.CardId == choice || action.Action.PlayerId == choice)
                return action;
        }
        throw new InvalidOperationException($"capture action not found: {kind} {choice}");
    }

    private static object CaptureMessage(
        RuntimeV4ExpertRestActionSupport support,
        RuntimeV4ExpertRestContext context,
        string step,
        RuntimeV4ExpertRestRequest request)
    {
        Check(RuntimeV4ExpertRestActionCodec.TrySerializeRequest(request, out string body,
            out string error), $"capture request failed to serialize: {error}");
        (int Status, string Response) accepted = support.Handle(context, body, out int status);
        Check(status == accepted.Status && status == 200, "capture admission failed");
        using (JsonDocument acceptedDocument = JsonDocument.Parse(accepted.Response))
            Check(acceptedDocument.RootElement.GetProperty("status").GetString() == "accepted",
                "capture admission did not return accepted");
        (int Status, string Response) settled = support.Handle(context,
            request.Operation.OperationId, out status);
        Console.WriteLine($"settled {step}: status={status}");
        Check(status == settled.Status && status == 200, "capture reconciliation failed");
        Check(RuntimeV4ExpertRestActionCodec.TryValidateResponse(settled.Response, context,
            out error), $"capture response failed consumer validation: {error}");
        return new
        {
            step,
            request = Parse(body),
            response = Parse(settled.Response)
        };
    }

    private static void WriteFixture(
        string directory,
        string fileName,
        string fixtureId,
        string lifecycle,
        IReadOnlyList<object> messages)
    {
        object fixture = new
        {
            fixture_version = "runtime-v4-expert-rest-action-producer-fixture-v1",
            protocol_version = RuntimeV4ExpertRestActionContract.ProtocolVersion,
            profile = RuntimeV4ExpertRestActionContract.Profile,
            schema_digest = RuntimeV4ExpertRestActionContract.SchemaDigest,
            provenance = new
            {
                artifact = RuntimeV4ExpertRestActionContract.Artifact,
                source = RuntimeV4ExpertRestActionContract.SchemaSource,
                generator = RuntimeV4ExpertRestActionContract.Generator
            },
            producer = new
            {
                kind = "managed_source_only",
                implementation = "experiments/managed-rust-interop/game-loader/RuntimeV4ExpertRestActionSupport.cs",
                host = "synthetic SmithHost/MendHost",
                capture = "RuntimeV4ExpertRestActionSupport.Handle action admission and operation reconciliation"
            },
            consumer_context = "stateful operation store retains selector admission catalog across response observations",
            fixture_id = fixtureId,
            lifecycle,
            messages
        };
        string path = Path.Combine(directory, fileName);
        File.WriteAllText(path, JsonSerializer.Serialize(fixture, FixtureJsonOptions) + Environment.NewLine);
        Console.WriteLine($"captured {path}");
    }

    private static JsonElement Parse(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json,
            new JsonDocumentOptions { MaxDepth = 24 });
        return document.RootElement.Clone();
    }
}
