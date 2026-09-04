// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using System.Text.Json.Nodes;
using AiAscension.Sts2GameMod.Runtime;

ModEntry.CheckContract("""
    {"protocol_version":"runtime-v1","schema_digest":"a76086d7a68668fd4cff53999369d2b450b0d6623827393882f458f2aa1f93eb",
    "provenance":{"artifact":"sts2-protocol/runtime-v1","source":"schemas/runtime-v1.schema.json","generator":"hand-authored"},
    "correlation_id":"correlation","instance_id":"instance","session_id":"session","lease_id":"lease",
    "lease_epoch":1,"generation":0,"kind":"action_request","observation":null,
    "action":{"action_id":"show_runtime_probe"},"status":null,"error_code":null,"effect_witness":null}
    """);
Console.WriteLine("Managed runtime-v1 contract regressions passed");

namespace AiAscension.Sts2GameMod.Runtime
{
    public static partial class ModEntry
    {
        internal static void CheckContract(string fixture)
        {
            Console.WriteLine(fixture.ReplaceLineEndings(""));
            JsonObject request = JsonNode.Parse(fixture)!.AsObject();
            RuntimeContext context = new(request["instance_id"]!.GetValue<string>(), "caller",
                request["session_id"]!.GetValue<string>(), request["lease_id"]!.GetValue<string>(),
                request["lease_epoch"]!.ToJsonString(), request["correlation_id"]!.GetValue<string>());
            ulong initial = request["generation"]!.GetValue<ulong>();
            _runtimeGeneration = initial;
            foreach (string field in new[] { "generation", "lease_epoch" })
            {
                foreach (string value in new[] { "null", "\"1\"", "true", "{}", "[]", "-1", "1.5", "9007199254740992" })
                {
                    JsonObject changed = request.DeepClone().AsObject();
                    changed[field] = JsonNode.Parse(value);
                    Reject(changed.ToJsonString(), context, field + value);
                }
            }
            foreach (string field in new[] { "observation", "action", "status", "error_code", "effect_witness", "provenance", "lease_epoch" })
            {
                JsonObject changed = request.DeepClone().AsObject();
                changed.Remove(field);
                Reject(changed.ToJsonString(), context, "missing " + field);
            }
            foreach (string objectPath in new[] { "", "action", "provenance" })
            {
                JsonObject changed = request.DeepClone().AsObject();
                JsonObject nested = objectPath.Length == 0 ? changed : changed[objectPath]!.AsObject();
                nested["extra"] = 1;
                Reject(changed.ToJsonString(), context, "extra field " + objectPath);
            }
            foreach (string field in new[] { "observation", "status", "error_code", "effect_witness" })
            {
                JsonObject changed = request.DeepClone().AsObject();
                changed[field] = "not null";
                Reject(changed.ToJsonString(), context, "nonnull " + field);
            }
            string wire = request.ToJsonString();
            Reject(wire.Insert(1, "\"generation\":" + initial + ","), context, "duplicate top-level");
            Reject(wire.Replace("\"action_id\":", "\"action_id\":\"show_runtime_probe\",\"action_id\":", StringComparison.Ordinal),
                context, "duplicate action");
            Reject(wire.Replace("\"artifact\":", "\"artifact\":\"sts2-protocol/runtime-v1\",\"artifact\":", StringComparison.Ordinal),
                context, "duplicate provenance");
            JsonObject wrongEpoch = request.DeepClone().AsObject();
            wrongEpoch["lease_epoch"] = request["lease_epoch"]!.GetValue<ulong>() + 1;
            Reject(wrongEpoch.ToJsonString(), context, "body/header epoch mismatch");
            Reject("{", context, "malformed JSON");
            Reject("[]", context, "root array");
            foreach (string value in new[] { "invalid epoch", "9007199254740992", "-1", "+1" })
            {
                RuntimeContext invalid = new(context.InstanceId, context.CallerId, context.SessionId,
                    context.LeaseId, value, context.CorrelationId);
                Reject(wire, invalid, "invalid context epoch");
            }
            foreach (string value in new[] { "", "space invalid", "unicode-λ", new string('a', 129) })
            {
                RuntimeContext invalid = new(context.InstanceId, value, context.SessionId,
                    context.LeaseId, context.LeaseEpoch, context.CorrelationId);
                Reject(wire, invalid, "invalid caller context");
            }
            if (_hostCalls != 0) throw new InvalidOperationException("rejection mutated host");
            var accepted = ProcessRuntimeWork(new RuntimeWork(2, context, wire));
            Console.WriteLine(accepted.Response);
            using JsonDocument response = JsonDocument.Parse(accepted.Response);
            if (accepted.Status != 200 || _hostCalls != 1
                || response.RootElement.GetProperty("generation").GetUInt64() != initial + 1
                || response.RootElement.GetProperty("effect_witness").GetProperty("generation").GetUInt64() != initial + 1)
                throw new InvalidOperationException("valid action acceptance");
            var stale = ProcessRuntimeWork(new RuntimeWork(2, context, wire));
            Console.WriteLine(stale.Response);
            if (stale.Status != 409 || _hostCalls != 1)
                throw new InvalidOperationException("stale generation executed twice");
            _runtimeGeneration = initial;
            _runtimeActionCount = 1024;
            var limited = ProcessRuntimeWork(new RuntimeWork(2, context, wire));
            Console.WriteLine(limited.Response);
            Console.WriteLine(ProcessRuntimeWork(new RuntimeWork(1, context, "")).Response);
            if (limited.Status != 409 || _hostCalls != 1)
                throw new InvalidOperationException("counter bound mutated host");
            _runtimeActionCount = 0;
            _retainOverlay = false;
            var uncertain = ProcessRuntimeWork(new RuntimeWork(2, context, wire));
            if (uncertain != (503, "{\"error_code\":\"runtime_probe_outcome_unknown\"}") || _hostCalls != 2)
                throw new InvalidOperationException("missing post-effect witness falsely claimed rejection");
        }

        private static void Reject(string wire, RuntimeContext context, string name)
        {
            if (ProcessRuntimeWork(new RuntimeWork(2, context, wire)).Status != 400)
                throw new InvalidOperationException(name);
        }
    }
}
