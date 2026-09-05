// SPDX-License-Identifier: MIT
using System.Text.Json;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.GameActions;
namespace AiAscension.Sts2GameMod.Runtime;
public static partial class ModEntry
{
    private const string LogPrefix = "synthetic";
    private static RuntimeContext Context(string correlation = "corr") => new("instance", "caller", "session", "lease", "1", correlation);
    private static string Request(string op = "op", string correlation = "corr") => JsonSerializer.Serialize(new Dictionary<string, object?>
    {
        ["protocol_version"] = RuntimeV2ProtocolVersion,
        ["schema_digest"] = RuntimeV2SchemaDigest,
        ["provenance"] = RuntimeV2Provenance(),
        ["correlation_id"] = correlation,
        ["instance_id"] = "instance",
        ["session_id"] = "session",
        ["lease_id"] = "lease",
        ["lease_epoch"] = 1,
        ["generation"] = 0,
        ["kind"] = "action_request",
        ["operation_id"] = op,
        ["observation"] = null,
        ["action"] = new { action_id = "end_turn" },
        ["status"] = null,
        ["error_code"] = null,
        ["effect_witness"] = null
    });
    private static void Reset()
    {
        RunManager.Instance = new();
        CombatManager.Instance = new();
        RunManager.Instance.State.Player.PlayerCombatState.Hand.Cards.Add(new CardModel());
        RuntimeV2Operations.Clear();
        _runtimeV2Binding = null;
        _runtimeV2Pending = null;
        _runtimeV2HostBaseline = false;
        _runtimeGeneration = 0;
        InitializeRuntimeV3Gameplay();

    }

    private static string Status((int Status, string Response) response)
    {
        using JsonDocument document = JsonDocument.Parse(response.Response);
        return document.RootElement.GetProperty("status").GetString()!;
    }
    private static void Check(bool test, string message) { if (!test) throw new InvalidOperationException(message); Console.WriteLine("PASS: " + message); }
    public static void Main()
    {
        Reset(); var result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()));
        RunManager.Instance.State.Player.PlayerCombatState.TurnNumber++;
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Operation, Context(), "op"));
        Check(Status(result) == "unknown", "unrelated next turn cannot settle unexecuted queued end-turn action");
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context("retry"), Request(correlation: "retry")));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "same operation/action with new request correlation replays");
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request().Replace(",", ", ", StringComparison.Ordinal)));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "JSON formatting does not change operation identity");
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(),
            Request().Replace("\"generation\":0", "\"generation\":1", StringComparison.Ordinal)));
        Check(Status(result) == "rejected" && result.Response.Contains("idempotency_conflict", StringComparison.Ordinal),
            "same operation with a changed generation conflicts without redispatch");
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request("next")));
        Check(Status(result) == "rejected" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "unknown operation prevents a second mutation");
        Reset(); RunManager.Instance.ActionQueueSynchronizer.ThrowAfterEnqueue = true;
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "exception after enqueue remains unknown");
        Reset();
        var otherOwner = new RuntimeContext("instance", "caller2", "session2", "lease2", "2", "corr");
        Check(TryAuthorizeRuntimeV2Context(Context(), out _) && !TryAuthorizeRuntimeV2Context(otherOwner, out _),
            "another owner cannot adopt the bound host");
        Reset();
        ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2State, Context(), ""));
        RunManager.Instance = new();
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request()));
        Check(Status(result) == "rejected" && result.Response.Contains("stale_generation", StringComparison.Ordinal), "v2 new run invalidates old generation despite identical turn number");
        Reset();
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Action, Context(), Request("run/operation")));
        Check(Status(result) == "unknown", "bounded slash operation identity is admitted");
        result = ProcessRuntimeWork(new(RuntimeRequestKindRuntimeV2Operation, Context(), "run/operation"));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "slash operation identity is reconciled without new mutation");
        Reset();
        var queued = RuntimeQueue.Enqueue(new RuntimeWork(RuntimeRequestKindRuntimeV2Action, Context(), Request()))
            ?? throw new InvalidOperationException("combined queue admission failed");
        ProcessRuntimeQueue();
        result = RuntimeQueue.Wait(queued, TimeSpan.Zero);
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1,
            "shared production queue dispatches candidate exactly once and preserves uncertainty");
        Reset();
        var expired = RuntimeQueue.Enqueue(new RuntimeWork(RuntimeRequestKindRuntimeV2Action, Context(), Request()), TimeSpan.Zero)
            ?? throw new InvalidOperationException("combined expired queue admission failed");
        ProcessRuntimeQueue();
        Check(RuntimeQueue.Wait(expired, TimeSpan.Zero).Status == 504
            && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 0,
            "shared queue removes expired candidate before host dispatch");
        CheckSharedGameplayBoundary();
    }
}
