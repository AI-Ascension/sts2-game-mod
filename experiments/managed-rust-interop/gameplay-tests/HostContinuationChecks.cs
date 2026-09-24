// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

/// <summary>
/// Synthetic checks for the host-owned <c>continue_run</c> offer and dispatch lane. No game,
/// profile, save, or process is read; the native resumable-run detection stays unverified here.
/// </summary>
internal static class HostContinuationChecks
{
    internal static void Run()
    {
        ResumableRunIsDetectedOnlyThroughNativeGuards();
        CatalogAdmitsBothAcceptedShapesAndRefusesTheRest();
        CodecEmitsOnlyTheAgreedShapes();
        DispatchUsesTheOfferedContinuationAndRefusesTheRest();
        PayloadShapesAreRefusedBeforeAnyMutation();
    }

    private static void ResumableRunIsDetectedOnlyThroughNativeGuards()
    {
        var resumable = new RuntimeV3GameplayContinuation.Availability(true, 1, true, false);
        Require(RuntimeV3GameplayContinuation.IsResumable(resumable),
            "an initialized profile with a saved run is resumable");
        foreach (RuntimeV3GameplayContinuation.Availability unavailable in new[]
        {
            resumable with { ProfileInitialized = false },
            resumable with { ProfileId = 0 },
            resumable with { ProfileId = 4 },
            resumable with { HasRunSave = false },
            resumable with { RunInProgress = true },
            default
        })
        {
            Require(!RuntimeV3GameplayContinuation.IsResumable(unavailable),
                "an incomplete or in-progress native summary must not offer a continuation");
        }
        LegalActionReference action = RuntimeV3GameplayContinuation.Create(7);
        Require(action.ActionId == "continue_run:7" && action.Kind == "continue_run"
            && action.Value is null && action.TargetId is null && action.Generation == 7
            && action.Validate(out _),
            "the offered continuation matches the accepted argument-free shape");
    }

    private static void CatalogAdmitsBothAcceptedShapesAndRefusesTheRest()
    {
        var bare = new LegalActionReference("continue_run:2", "continue_run", null, null, 2);
        var named = new LegalActionReference("continue_run:2:profile1", "continue_run", "profile1", null, 2);
        Require(bare.Validate(out _) && named.Validate(out _),
            "both accepted continuation shapes validate");
        Require(!(named with { TargetId = "enemy:1" }).Validate(out _),
            "a continuation target is refused");
        Require(!(named with { Value = "profile 1" }).Validate(out _),
            "a non-identity run discriminator is refused");
        Require(!(bare with { Generation = RuntimeV3GameplayContract.MaxGeneration + 1 }).Validate(out _),
            "an out-of-bound generation is refused");
        Require(LegalActionCatalog.TryCreate(2, new[] { bare, named }, out _),
            "a catalog may offer both continuation shapes");
        Require(!LegalActionCatalog.TryCreate(2, new[] { bare, named with { ActionId = "continue_run:2" } }, out _),
            "a duplicate continuation identity is refused");
        Require(!new LegalActionReference("resume:2", "resume_run", null, null, 2).Validate(out _),
            "an unowned continuation kind is not part of the catalog");
    }

    private static void CodecEmitsOnlyTheAgreedShapes()
    {
        RuntimeV3GameplayObservation setup = RuntimeV3GameplayFixtures.CombatObservation(2) with
        {
            State = RuntimeV3GameplayState.Setup,
            StateValues = new[] { "IRONCLAD" },
            IsActionable = true,
            InputEnabled = true,
            ModalBlocking = false
        };
        Require(RuntimeV3GameplayCodec.TrySerialize(setup,
                new[] { new LegalActionReference("continue_run:2", "continue_run", null, null, 2) },
                out string bareJson, out _),
            "the argument-free continuation serializes");
        JsonElement bareAction = FirstAction(bareJson);
        Require(bareAction.GetProperty("kind").GetString() == "continue_run"
            && !bareAction.TryGetProperty("run_id", out _),
            "the argument-free continuation never emits a null discriminator");
        Require(RuntimeV3GameplayCodec.TrySerialize(setup,
                new[] { new LegalActionReference("continue_run:2:profile1", "continue_run", "profile1", null, 2) },
                out string namedJson, out _),
            "the discriminated continuation serializes");
        JsonElement namedAction = FirstAction(namedJson);
        Require(namedAction.GetProperty("run_id").GetString() == "profile1",
            "the discriminated continuation carries its run identity");
    }

    private static void DispatchUsesTheOfferedContinuationAndRefusesTheRest()
    {
        var source = new FakeHost { ActionKind = "continue_run", ActionId = "continue_run:2" };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        using (JsonDocument catalog = Wire.Call(support, "legal_actions_request", 1, out int catalogStatus))
        {
            Require(catalogStatus == 200
                && catalog.RootElement.GetProperty("legal_actions")[0].GetProperty("action_id")
                    .GetString() == "continue_run:2",
                "the host-generated continuation identity is offered unchanged");
        }
        using JsonDocument dispatched = Wire.Call(support, "dispatch_action_request", 1,
            out int dispatchedStatus, actionKind: "continue_run",
            transform: body => body.Replace("\"action_id\":\"combat.end-turn\"",
                "\"action_id\":\"continue_run:2\"", StringComparison.Ordinal));
        Require(dispatchedStatus == 503 && Wire.Status(dispatched) == "unknown" && source.Dispatches == 1,
            "an offered continuation dispatches through the ordinary receipt path");
        using JsonDocument replay = Wire.Call(support, "dispatch_action_request", 1, out _,
            actionKind: "continue_run",
            transform: body => body.Replace("\"action_id\":\"combat.end-turn\"",
                "\"action_id\":\"continue_run:2\"", StringComparison.Ordinal));
        Require(Wire.Status(replay) == "unknown" && source.Dispatches == 1,
            "an unknown continuation replay does not mutate twice");
        source.Complete = true;
        using JsonDocument settled = Wire.Call(support, "wait_request", 1, out int settledStatus);
        Require(settledStatus == 200 && Wire.Status(settled) == "settled" && source.Dispatches == 1,
            "a read-only wait settles the independently completed continuation");
    }

    private static JsonElement FirstAction(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        return document.RootElement.GetProperty("legal_actions")[0].GetProperty("action").Clone();
    }

    private static void PayloadShapesAreRefusedBeforeAnyMutation()
    {
        var host = new FakeHost { ActionKind = "continue_run", ActionId = "continue_run:2" };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(host, new TestQueue());
        // The discriminated shape parses, but this synthetic host offers only the bare one, so it is
        // refused as not-current rather than as a malformed request.
        using (JsonDocument unoffered = Wire.Call(support, "dispatch_action_request", 1,
            out int unofferedStatus, actionKind: "continue_run",
            transform: body => body.Replace("\"kind\":\"continue_run\"",
                "\"kind\":\"continue_run\",\"run_id\":\"profile1\"", StringComparison.Ordinal)))
        {
            Require(unofferedStatus == 409 && Wire.Error(unoffered) == "action_not_current",
                "an unoffered discriminated continuation is refused as not-current");
        }
        foreach ((string fragment, string reason) in new[]
        {
            ("\"kind\":\"continue_run\",\"save_path\":\"profile1/saves/current_run.save\"", "save path"),
            ("\"kind\":\"continue_run\",\"run_id\":null", "null run identity"),
            ("\"kind\":\"continue_run\",\"run_id\":\"\"", "blank run identity"),
            ("\"kind\":\"continue_run\",\"run_id\":\"profile 1\"", "non-identity run identity"),
            ("\"kind\":\"continue_run\",\"run_id\":\"profile1\",\"seed\":\"1\"", "extra field")
        })
        {
            var malformedHost = new FakeHost { ActionKind = "continue_run", ActionId = "continue_run:2" };
            using JsonDocument refused = Wire.Call(
                RuntimeV3GameplaySupport.WithHost(malformedHost, new TestQueue()),
                "dispatch_action_request", 1, out int refusedStatus, actionKind: "continue_run",
                transform: body => body.Replace("\"kind\":\"continue_run\"", fragment,
                    StringComparison.Ordinal));
            Require(refusedStatus == 400 && malformedHost.Dispatches == 0,
                $"a continuation carrying a {reason} must be refused before any host mutation");
        }
        // A continuation identity with a start_run payload is not coerced onto the continuation.
        var mixedHost = new FakeHost();
        using (JsonDocument mixed = Wire.Call(RuntimeV3GameplaySupport.WithHost(mixedHost, new TestQueue()),
            "dispatch_action_request", 1, out int mixedStatus, actionKind: "start_run",
            transform: body => body.Replace("\"action_id\":\"combat.end-turn\"",
                "\"action_id\":\"continue_run:2\"", StringComparison.Ordinal)))
        {
            Require(mixedStatus == 400 && mixedHost.Dispatches == 0,
                "a continuation identity with a start_run payload is refused, not mapped to start_run");
        }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
