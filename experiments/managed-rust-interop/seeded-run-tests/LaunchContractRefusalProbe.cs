// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Source-only checks for the launch-contract refusal vocabulary. A refused launch contract used to
/// answer the same recovery code as a lane that never declared one, and the refusal also suppressed
/// the listener, so a runtime consumer could read neither the refusal nor its reason. The reason had
/// reached only <c>game.log</c> (sts2-game-mod#185).
/// </summary>
internal static class LaunchContractRefusalProbe
{
    private const string Unset = "launch_contract_refused_isolated_user_dir_unset";
    private const string Unresolved = "launch_contract_refused_isolated_user_dir_unresolved";
    private const string Mismatch = "launch_contract_refused_isolated_user_dir_mismatch";
    private const string Campaign = "launch_contract_refused_campaign_required";

    private static void Main()
    {
        LaunchContractRefusal.Clear();
        Check(ObservedCode(Request("state_request")) == LaunchContractRefusal.NotDeclared,
            "a lane that never declared a contract answers the unchanged recovery code");

        var reasons = new (string Reason, string Code)[]
        {
            (IsolatedUserDirectoryCheck.ExpectedUnset, Unset),
            (IsolatedUserDirectoryCheck.ResolvedUnset, Unresolved),
            (IsolatedUserDirectoryCheck.Mismatch, Mismatch),
            (LaunchContractRefusal.CampaignRequiredReason, Campaign),
        };

        var distinct = new HashSet<string> { LaunchContractRefusal.NotDeclared };
        foreach ((string reason, string code) in reasons)
        {
            LaunchContractRefusal.Clear();
            LaunchContractRefusal.Record(reason);
            Check(ObservedCode(Request("state_request")) == code,
                $"the {reason} refusal is answered as {code}");
            Check(LaunchContractRefusal.IsRefusal(code)
                    && !LaunchContractRefusal.IsRefusal(LaunchContractRefusal.NotDeclared),
                $"the {reason} refusal code is classifiable and not the missing-contract code");
            Check(distinct.Add(code), $"the {reason} refusal code is distinct from every other code");
        }

        string resolved = Path.Combine(Path.GetTempPath(), "sts2-refusal-probe-resolved");
        string declared = Path.Combine(Path.GetTempPath(), "sts2-refusal-probe-declared");
        IsolatedUserDirectoryDecision mismatch = IsolatedUserDirectoryCheck.Evaluate(resolved, declared);
        LaunchContractRefusal.Clear();
        LaunchContractRefusal.Record(mismatch.Reason);
        string body = Request("state_request");
        Check(ObservedCode(body) == Mismatch,
            "a measured mismatch is answered as the mismatch refusal");
        Check(!body.Contains(resolved, StringComparison.Ordinal)
                && !body.Contains(declared, StringComparison.Ordinal)
                && !body.Contains(IsolatedUserDirectoryCheck.Refusal, StringComparison.Ordinal),
            "the refusal answer names no directory and carries no diagnostic, which stays in game.log");

        LaunchContractRefusal.Clear();
        Check(DispatchCode() == LaunchContractRefusal.NotDeclared + "_or_invalid_action",
            "an unconfigured dispatch keeps its unchanged compound code");
        LaunchContractRefusal.Record(IsolatedUserDirectoryCheck.Mismatch);
        Check(DispatchCode() == Mismatch + "_or_invalid_action",
            "a refused launch names itself in the dispatch answer too");

        LaunchContractRefusal.Clear();
        LaunchContractRefusal.Record("a_reason_this_vocabulary_does_not_name");
        Check(ObservedCode(Request("state_request"))
                == "launch_contract_refused_a_reason_this_vocabulary_does_not_name",
            "an unnamed but well-formed refusal reason still yields a classifiable refusal code");

        LaunchContractRefusal.Clear();
        LaunchContractRefusal.Record("a reason that cannot be a wire identity");
        Check(ObservedCode(Request("state_request")) == LaunchContractRefusal.Unclassified,
            "a reason that cannot be a wire identity fails closed to the unclassified refusal");

        LaunchContractRefusal.Clear();
        Check(LaunchContractRefusal.UnconfiguredCode == LaunchContractRefusal.NotDeclared,
            "clearing the record restores the missing-contract code");

        Console.WriteLine("launch contract refusal checks passed");
    }

    private static string Request(string kind)
    {
        string body = RuntimeV3GameplaySupport.Unconfigured().Handle(
            "instance-1", "session-1", "lease-1", "request-1", "1", Envelope(kind), out int status);
        Check(status == 503, "a launch that cannot serve answers the bounded unavailable status");
        return body;
    }

    private static string ObservedCode(string body)
    {
        using JsonDocument document = JsonDocument.Parse(body);
        JsonElement state = document.RootElement.GetProperty("observation").GetProperty("state");
        Check(state.GetProperty("state").GetString() == "recovery",
            "the answer is still a recovery state");
        return state.GetProperty("code").GetString() ?? string.Empty;
    }

    private static string DispatchCode()
    {
        using JsonDocument document = JsonDocument.Parse(Request("dispatch_action_request"));
        return document.RootElement.GetProperty("error_code").GetString() ?? string.Empty;
    }

    private static string Envelope(string kind) => JsonSerializer.Serialize(
        new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplayContract.SchemaDigest,
            ["provenance"] = new
            {
                artifact = RuntimeV3GameplayContract.Artifact,
                source = RuntimeV3GameplayContract.SchemaSource,
                generator = RuntimeV3GameplayContract.Generator
            },
            ["correlation_id"] = "request-1",
            ["instance_id"] = "instance-1",
            ["session_id"] = "session-1",
            ["lease_id"] = "lease-1",
            ["lease_epoch"] = 1,
            ["generation"] = 1,
            ["kind"] = kind,
            ["state_id"] = kind == "dispatch_action_request" ? "combat-1" : null,
            ["operation_id"] = kind == "dispatch_action_request" ? "operation-1" : null,
            ["observation"] = null,
            ["legal_actions"] = null,
            ["action"] = kind == "dispatch_action_request"
                ? new { action_id = "combat.end-turn", action = new { kind = "end_turn" } }
                : null,
            ["status"] = null,
            ["transition"] = null,
            ["error_code"] = null,
            ["wait_for_millis"] = null,
            ["wait_outcome"] = null,
            ["recovery"] = null
        });

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }

        Console.WriteLine("PASS: " + message);
    }
}
