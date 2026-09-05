// SPDX-License-Identifier: MIT

using System.Text.Json;
using AiAscension.Sts2GameMod.GameplayTests;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string SemanticRequest(string kind, string operation = "semantic-op", ulong generation = 1)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplayContract.SchemaDigest,
            ["provenance"] = new { artifact = RuntimeV3GameplayContract.Artifact,
                source = RuntimeV3GameplayContract.SchemaSource, generator = RuntimeV3GameplayContract.Generator },
            ["correlation_id"] = "corr", ["instance_id"] = "instance",
            ["session_id"] = "session", ["lease_id"] = "lease", ["lease_epoch"] = 1,
            ["generation"] = generation, ["kind"] = kind,
            ["state_id"] = kind == "dispatch_action_request" ? "combat-1" : null,
            ["operation_id"] = kind is "dispatch_action_request" or "wait_request" ? operation : null,
            ["observation"] = null, ["legal_actions"] = null,
            ["action"] = kind == "dispatch_action_request"
                ? new { action_id = "combat.end-turn", action = new { kind = "end_turn" } } : null,
            ["status"] = null, ["transition"] = null, ["error_code"] = null,
            ["wait_for_millis"] = kind == "wait_request" ? (int?)1 : null,
            ["wait_outcome"] = null,
            ["recovery"] = kind == "recover_request" ? new { kind = "reconcile", operation_id = operation } : null
        });
    }

    private static (int Status, string Response) Semantic(string kind, string operation = "semantic-op") =>
        ProcessRuntimeWork(new RuntimeWork(RuntimeRequestKindGameplay, Context(), SemanticRequest(kind, operation)));

    private static void CheckSharedGameplayBoundary()
    {
        CheckSharedIdentity();
        CheckPendingV2BlocksSemantic();
        CheckPendingSemanticBlocksV2();
        CheckQueuedSemanticAdmission();
    }

    private static void CheckSharedIdentity()
    {
        RuntimeContext[] foreignContexts =
        {
            new("other-instance", "caller", "session", "lease", "1", "corr"),
            new("instance", "other-caller", "session", "lease", "1", "corr"),
            new("instance", "caller", "other-session", "lease", "1", "corr"),
            new("instance", "caller", "session", "other-lease", "1", "corr"),
            new("instance", "caller", "session", "lease", "2", "corr")
        };
        foreach (RuntimeContext foreign in foreignContexts)
        {
            Reset();
            ConfigureRuntimeV3Gameplay(new FakeHost(), new TestQueue());
            Check(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2State, Context(), "")).Status == 200
                && Semantic("state_request").Status == 200, "v2 state and semantic state use distinct callback kinds");
            var denied = ProcessRuntimeWork(new(RuntimeRequestKindGameplay, foreign, SemanticRequest("state_request")));
            Check(denied.Status == 409, "v2-bound identity fences semantic requests before host access");
            Reset();
            ConfigureRuntimeV3Gameplay(new FakeHost(), new TestQueue());
            Check(Semantic("state_request").Status == 200, "semantic profile can bind shared authority first");
            denied = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2State, foreign, ""));
            Check(denied.Status == 409, "semantic-bound identity fences v2 requests before host access");
        }
    }

    private static void CheckPendingV2BlocksSemantic()
    {
        Reset();
        var source = new FakeHost();
        ConfigureRuntimeV3Gameplay(source, new TestQueue());
        Check(Status(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()))) == "unknown",
            "v2 callback kind4 retains uncertain end-turn mutation");
        Check(Status(Semantic("dispatch_action_request")) == "rejected" && source.Dispatches == 0,
            "unresolved v2 action fences semantic mutation");
        Check(Semantic("state_request").Status == 200, "uncertain v2 mutation permits semantic observation");
        Check(Status(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Operation, Context(), "op"))) == "unknown",
            "v2 callback kind5 reconciles without mutation");
        Check(Status(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()))) == "unknown"
            && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1,
            "v2 exact retry survives cross-profile pending checks without redispatch");
    }

    private static void CheckPendingSemanticBlocksV2()
    {
        Reset();
        var source = new FakeHost();
        ConfigureRuntimeV3Gameplay(source, new TestQueue());
        Check(Status(Semantic("dispatch_action_request")) == "unknown", "semantic callback kind6 retains uncertain mutation");
        Check(Status(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()))) == "rejected"
            && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 0,
            "unknown semantic action fences v2 host mutation");
        Check(Status(Semantic("dispatch_action_request")) == "unknown" && source.Dispatches == 1,
            "semantic exact retry preserves its receipt without redispatch");
        Check(Semantic("state_request").Status == 200, "uncertain semantic mutation permits observation");
        source.Complete = true;
        Check(Status(Semantic("recover_request")) == "settled", "independent semantic completion reconciles while fenced");
        Check(Status(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request("after-semantic")))) == "unknown"
            && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1,
            "settled semantic receipt releases v2 admission");
    }

    private static void CheckQueuedSemanticAdmission()
    {
        Reset();
        var source = new FakeHost();
        var queue = new TestQueue { Deferred = true };
        ConfigureRuntimeV3Gameplay(source, queue);
        Check(Status(Semantic("dispatch_action_request")) == "accepted", "queued semantic receipt reserves mutation admission");
        Check(Status(ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()))) == "rejected"
            && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 0,
            "accepted semantic action excludes v2 before semantic dispatch");
        queue.Run();
        Check(source.Dispatches == 1 && Status(Semantic("wait_request")) == "unknown",
            "queued semantic mutation dispatches once and remains uncertain");
        // Exercise delayed external admission change independently of production serialization.
        bool admitted = true;
        source = new FakeHost();
        queue = new TestQueue { Deferred = true };
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, queue, () => admitted);
        support.Handle("instance", "session", "lease", "corr", "1", SemanticRequest("dispatch_action_request"), out _);
        admitted = false;
        queue.Run();
        string response = support.Handle("instance", "session", "lease", "corr", "1", SemanticRequest("recover_request"), out _);
        Check(source.Dispatches == 0 && response.Contains("operation_in_progress", StringComparison.Ordinal)
            && !support.HasPendingMutation, "queued semantic action rechecks shared admission before mutation");
    }
}
