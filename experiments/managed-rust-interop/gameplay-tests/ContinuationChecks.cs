// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class ContinuationChecks
{
    internal static void Run()
    {
        foreach (string title in new[] { "Shrug It Off", "Bash+", "漢字", new string('a', 900) })
        {
            string identity = RuntimeV3GameplayChoiceIdentity.Card("card:11", title);
            Require(RuntimeV3GameplayContract.IsIdentity(identity), "card choice obeys the wire identity bound");
            var observation = RuntimeV3GameplayFixtures.CombatObservation(1) with
            {
                State = RuntimeV3GameplayState.Selection, StateValues = new[] { identity }
            };
            Require(observation.Validate(out _), "selection choices validate through the real observation boundary");
            var action = new LegalActionReference($"select_card:{RuntimeV3GameplayContract.MaxGeneration}:{identity}",
                "select_card", identity, null, RuntimeV3GameplayContract.MaxGeneration);
            Require(action.Validate(out _), "choice label leaves room for the generation-bound action identity");
        }
        foreach (string kind in new[] { "proceed", "confirm_selection", "cancel_selection" })
        {
            var source = new FakeHost { ActionKind = kind };
            RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
            using JsonDocument invalid = Wire.Call(support, "dispatch_action_request", 1,
                out int invalidStatus, actionKind: kind, transform: body => body.Replace(
                    $"\"kind\":\"{kind}\"", $"\"kind\":\"{kind}\",\"choice_id\":\"extra\"",
                    StringComparison.Ordinal));
            Require(invalidStatus == 400 && source.Dispatches == 0, "extra argument reaches no host mutation");
            using JsonDocument first = Wire.Call(support, "dispatch_action_request", 1,
                out int firstStatus, actionKind: kind);
            Require(firstStatus == 503 && Wire.Status(first) == "unknown" && source.Dispatches == 1,
                "continuation admission without an independent witness stays unknown");
            using JsonDocument replay = Wire.Call(support, "dispatch_action_request", 1,
                out _, actionKind: kind);
            Require(Wire.Status(replay) == "unknown" && source.Dispatches == 1,
                "unknown continuation replay does not mutate twice");
            source.Complete = true;
            using JsonDocument settled = Wire.Call(support, "wait_request", 1, out int status);
            Require(status == 200 && Wire.Status(settled) == "settled" && source.Dispatches == 1,
                "read-only continuation wait requires the bound completion witness");
        }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
