// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class Program
{
    private static int Main()
    {
        try
        {
            CapturesOwnedValuesAndRetainsOccurrenceHandles();
            RotatesRunIncarnationOnInvalidation();
            RejectsDuplicateAndMissingRequiredFields();
            PreservesExplicitUnavailableStatuses();
            EnforcesBoundsBeforePublishing();
            Console.WriteLine("LiveCardSourceProbe: PASS");
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine(exception.Message);
            return 1;
        }
    }

    private static void CapturesOwnedValuesAndRetainsOccurrenceHandles()
    {
        var registry = new LiveCardSnapshotRegistry();
        var first = new object();
        var second = new object();
        var run = new object();
        LiveCardCapturedSnapshot snapshot = Capture(registry, run, "run-a",
            Input(first, "card:alpha", LiveCardZone.Hand, 0),
            Input(second, "card:alpha", LiveCardZone.Deck, 3));
        Check(snapshot.Available && snapshot.Cards.Count == 2,
            "captures a bounded owned card snapshot");
        Check(snapshot.Cards[0].InstanceId != snapshot.Cards[1].InstanceId,
            "same definition receives distinct occurrence identities");
        Check(snapshot.Cards[0].DefinitionId.Status == LiveCardFieldStatus.Available
            && snapshot.Cards[0].DefinitionId.Value == "card:alpha",
            "copies the host definition identity");
        Check(snapshot.Cards[0].Location.Value.Zone == LiveCardZone.Hand
            && snapshot.Cards[1].Location.Value.Position == 3,
            "copies source zone and position evidence");
        Check(snapshot.StateGeneration == 42,
            "preserves the gameplay observation generation separately from the source epoch");
        Check(registry.IsCurrent(snapshot, "instance:1", "manifest:1"),
            "the captured owner-local read fence is current");

        LiveCardCapturedSnapshot next = Capture(registry, run, "run-a",
            Input(first, "card:alpha", LiveCardZone.Discard, 0),
            Input(second, "card:alpha", LiveCardZone.Deck, 2));
        Check(next.Available && next.Epoch > snapshot.Epoch
            && next.Cards[0].InstanceId == snapshot.Cards[0].InstanceId,
            "same observed host objects retain occurrence handles across snapshots");
        Check(registry.IsCurrent(next, "instance:1", "manifest:1")
            && !registry.IsCurrent(snapshot, "instance:1", "manifest:1")
            && !registry.IsCurrent(next, "instance:other", "manifest:1")
            && !registry.IsCurrent(next, "instance:1", "manifest:other"),
            "only the newest owner-local snapshot fence remains current");
    }

    private static void RotatesRunIncarnationOnInvalidation()
    {
        var registry = new LiveCardSnapshotRegistry();
        var card = new object();
        var run = new object();
        LiveCardCapturedSnapshot first = Capture(registry, run, "same-seed",
            Input(card, "card:alpha", LiveCardZone.Hand, 0));
        LiveCardCapturedSnapshot changedRun = Capture(registry, new object(), "same-seed",
            Input(card, "card:alpha", LiveCardZone.Hand, 0));
        Check(first.RunId != changedRun.RunId && first.Cards[0].InstanceId
            != changedRun.Cards[0].InstanceId,
            "a changed host run object rotates the source incarnation even for a repeated seed");
        registry.Invalidate();
        LiveCardCapturedSnapshot second = Capture(registry, new object(), "same-seed",
            Input(card, "card:alpha", LiveCardZone.Hand, 0));
        Check(first.RunId != second.RunId && first.SnapshotId != second.SnapshotId,
            "source invalidation rotates run and snapshot identity even for a repeated seed");
        Check(first.Cards[0].InstanceId != second.Cards[0].InstanceId,
            "invalidated host-object handles cannot be reused by a later run");
        Check(second.Epoch > first.Epoch,
            "the owner epoch remains monotonic across lifecycle rotation");

        LiveCardCapturedSnapshot fresh = Capture(new LiveCardSnapshotRegistry(),
            new object(), "same-seed", Input(new object(), "card:alpha", LiveCardZone.Hand, 0));
        Check(first.RunId != fresh.RunId && first.Cards[0].InstanceId != fresh.Cards[0].InstanceId,
            "a recreated source registry receives a fresh lifetime nonce");
    }

    private static void RejectsDuplicateAndMissingRequiredFields()
    {
        var registry = new LiveCardSnapshotRegistry();
        var card = new object();
        LiveCardCapturedSnapshot duplicate = Capture(registry, new object(), "duplicate",
            Input(card, "card:alpha", LiveCardZone.Hand, 0),
            Input(card, "card:alpha", LiveCardZone.Deck, 0));
        Check(!duplicate.Available && duplicate.UnavailableReason == "duplicate_card_occurrence",
            "duplicate host references are rejected before publication");

        LiveCardCaptureInput missingManifest = Input(new object(), "card:alpha",
            LiveCardZone.Hand, 0) with
        {
            DefinitionManifest = LiveCardField<string>.NotObserved()
        };
        LiveCardCapturedSnapshot missing = registry.Capture(
            "instance:1", new object(), "missing", "manifest:1", 42,
            new[] { missingManifest });
        Check(!missing.Available && missing.UnavailableReason == "required_card_field_unavailable",
            "a missing required binding field fails closed");

        LiveCardCapturedSnapshot mismatched = registry.Capture(
            "instance:1", new object(), "mismatch", "manifest:1", 42,
            new[] { Input(new object(), "card:alpha", LiveCardZone.Hand, 0)
                with { DefinitionManifest = LiveCardField<string>.Available("manifest:other") } });
        Check(!mismatched.Available && mismatched.UnavailableReason == "content_manifest_mismatch",
            "card definitions cannot cross the capture manifest fence");
    }

    private static void PreservesExplicitUnavailableStatuses()
    {
        var registry = new LiveCardSnapshotRegistry();
        LiveCardCaptureInput input = Input(new object(), "card:alpha", LiveCardZone.Hand, 0);
        LiveCardCapturedSnapshot snapshot = Capture(registry, new object(), "unknown-fields", input);
        LiveCardCapturedCard card = snapshot.Cards[0];
        Check(card.BaseCost.Status == LiveCardFieldStatus.NotObserved
            && card.Modifiers.Status == LiveCardFieldStatus.NotObserved
            && card.Flags.Status == LiveCardFieldStatus.NotObserved
            && card.EffectParameters.Status == LiveCardFieldStatus.NotObserved,
            "unobservable host fields remain explicit not-observed statuses");
        Check(card.Modifiers.Value is null,
            "unobservable reference fields do not become empty collections");
    }

    private static void EnforcesBoundsBeforePublishing()
    {
        var registry = new LiveCardSnapshotRegistry();
        var cards = new List<LiveCardCaptureInput>(LiveCardSnapshotRegistry.MaxCards + 1);
        for (int index = 0; index <= LiveCardSnapshotRegistry.MaxCards; index++)
        {
            cards.Add(Input(new object(), $"card:{index}", LiveCardZone.Deck, index));
        }

        LiveCardCapturedSnapshot snapshot = registry.Capture(
            "instance:1", new object(), "bounded", "manifest:1", 42, cards);
        Check(!snapshot.Available && snapshot.UnavailableReason == "card_count_exceeded",
            "card-count bound rejects an oversized capture");

        LiveCardCaptureInput oversizedList = Input(
            new object(), "card:list", LiveCardZone.Deck, 0) with
        {
            Flags = LiveCardField<IReadOnlyList<string>>.Available(
                new[] { new string('x', LiveCardSnapshotRegistry.MaxIdentityBytes + 1) })
        };
        LiveCardCapturedSnapshot listResult = registry.Capture(
            "instance:1", new object(), "oversized-list", "manifest:1", 42,
            new[] { oversizedList });
        Check(!listResult.Available && listResult.UnavailableReason == "card_field_bound_exceeded",
            "auxiliary list values enforce a byte bound");

        LiveCardCaptureInput oversizedMap = Input(
            new object(), "card:map", LiveCardZone.Deck, 0) with
        {
            EffectParameters = LiveCardField<IReadOnlyDictionary<string, string>>.Available(
                new Dictionary<string, string>
                {
                    ["effect"] = new string('x', LiveCardSnapshotRegistry.MaxIdentityBytes + 1)
                })
        };
        LiveCardCapturedSnapshot mapResult = registry.Capture(
            "instance:1", new object(), "oversized-map", "manifest:1", 42,
            new[] { oversizedMap });
        Check(!mapResult.Available && mapResult.UnavailableReason == "card_field_bound_exceeded",
            "effect parameter values enforce a byte bound");
    }

    private static LiveCardCapturedSnapshot Capture(
        LiveCardSnapshotRegistry registry,
        object runHandle,
        string runKey,
        params LiveCardCaptureInput[] cards) =>
        registry.Capture("instance:1", runHandle, runKey, "manifest:1", 42, cards);

    private static LiveCardCaptureInput Input(
        object hostCard,
        string definition,
        LiveCardZone zone,
        int position) =>
        new(
            hostCard,
            LiveCardField<string>.Available(definition),
            LiveCardField<string>.Available("manifest:1"),
            LiveCardField<string>.Available("player:1"),
            LiveCardField<LiveCardLocation>.Available(new(zone, position)),
            LiveCardField<ushort>.Available(0),
            LiveCardField<string>.NotObserved(),
            LiveCardField<string>.NotObserved(),
            LiveCardField<string>.Available("Fixture Card"),
            LiveCardField<bool>.Available(false),
            LiveCardField<int>.NotObserved(),
            LiveCardField<int>.NotObserved(),
            LiveCardField<int>.Available(1),
            LiveCardField<IReadOnlyList<string>>.NotObserved(),
            LiveCardField<IReadOnlyList<string>>.NotObserved(),
            LiveCardField<IReadOnlyDictionary<string, string>>.NotObserved());

    private static void Check(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
        Console.WriteLine("PASS: " + message);
    }
}
