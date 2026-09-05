// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class Wire
{
    internal static JsonDocument Call(RuntimeV3GameplaySupport support, string kind, ulong generation,
        out int status, string stateId = "combat-1", string actionKind = "end_turn",
        string session = "session-1", ulong epoch = 1, Func<string, string>? transform = null,
        string recoveryKind = "reobserve")
    {
        var body = new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplayContract.SchemaDigest,
            ["provenance"] = new { artifact = RuntimeV3GameplayContract.Artifact,
                source = RuntimeV3GameplayContract.SchemaSource, generator = RuntimeV3GameplayContract.Generator },
            ["correlation_id"] = "request-1", ["instance_id"] = "instance-1",
            ["session_id"] = session, ["lease_id"] = "lease-1", ["lease_epoch"] = epoch,
            ["generation"] = generation, ["kind"] = kind,
            ["state_id"] = kind is "dispatch_action_request" or "legal_actions_request" ? stateId : null,
            ["operation_id"] = kind is "dispatch_action_request" or "wait_request" ? "operation-1" : null,
            ["observation"] = null, ["legal_actions"] = null,
            ["action"] = kind == "dispatch_action_request"
                ? new { action_id = "combat.end-turn", action = new { kind = actionKind } } : null,
            ["status"] = null, ["transition"] = null, ["error_code"] = null,
            ["wait_for_millis"] = kind == "wait_request" ? (int?)1 : null,
            ["wait_outcome"] = null,
            ["recovery"] = kind == "recover_request" ? new { kind = recoveryKind,
                operation_id = recoveryKind == "reconcile" ? "operation-1" : null } : null
        };
        string json = JsonSerializer.Serialize(body);
        return JsonDocument.Parse(support.Handle("instance-1", session, "lease-1", "request-1",
            epoch.ToString(CultureInfo.InvariantCulture), transform is null ? json : transform(json), out status));
    }

    internal static string? Status(JsonDocument document) => document.RootElement.GetProperty("status").GetString();
    internal static string? Error(JsonDocument document) => document.RootElement.GetProperty("error_code").GetString();
}
