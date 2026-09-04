// SPDX-License-Identifier: MIT
using System.Text.Json;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.GameActions;
namespace AiAscension.Sts2GameMod.Runtime;
public static partial class ModEntry
{
    private const int RuntimeAccepted = 200, RuntimeRejected = 409, RuntimeTooManyRequests = 429, RuntimeUnavailable = 503;
    private const uint RuntimeRequestKindRuntimeV2State = 3, RuntimeRequestKindRuntimeV2Action = 4, RuntimeRequestKindRuntimeV2Operation = 5, RuntimeRequestKindRuntimeV3State = 6, RuntimeRequestKindRuntimeV3Action = 7, RuntimeRequestKindRuntimeV3Operation = 8;
    private const string LogPrefix = "synthetic";
    private static ulong _runtimeGeneration;
    private static ulong ParseEpoch(string value) => ulong.Parse(value, System.Globalization.CultureInfo.InvariantCulture);
    private static string? StringField(JsonElement root, string path) { foreach (string part in path.Split('.')) { if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty(part, out root)) return null; } return root.ValueKind == JsonValueKind.String ? root.GetString() : null; }
    private static RuntimeContext Context(string correlation = "corr") => new("instance", "caller", "session", "lease", "1", correlation);
    private static string Request(string op = "op", string correlation = "corr") => JsonSerializer.Serialize(new Dictionary<string, object?>
    {
        ["protocol_version"] = RuntimeV3GameplayProtocolVersion,
        ["schema_digest"] = RuntimeV3GameplaySchemaDigest,
        ["provenance"] = RuntimeV3GameplayProvenance(),
        ["correlation_id"] = correlation,
        ["instance_id"] = "instance",
        ["session_id"] = "session",
        ["lease_id"] = "lease",
        ["lease_epoch"] = 1,
        ["generation"] = 0,
        ["kind"] = "action_request",
        ["operation_id"] = op,
        ["observation"] = null,
        ["action"] = new { action_id = "play_card", card_index = 0, target_id = (string?)null },
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
        RuntimeV3GameplayOperations.Clear();
        _runtimeV3GameplayBinding = null;
        _runtimeV3GameplayPending = null;
        _runtimeV3GameplayBaseline = false;
        _runtimeV3GameplayGeneration = 0;
        _runtimeV3GameplayLastSignature = "";
    }

    private static string Status((int Status, string Response) response)
    {
        using JsonDocument document = JsonDocument.Parse(response.Response);
        return document.RootElement.GetProperty("status").GetString()!;
    }
    private static void Check(bool test, string message) { if (!test) throw new InvalidOperationException(message); Console.WriteLine("PASS: " + message); }
    public static void Main()
    {
        Reset(); ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3State, Context(), ""));
        RunManager.Instance.State.Player.PlayerCombatState.Hand.Cards[0] = new CardModel { Identity = "replacement" };
        var result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request()));
        Check(Status(result) == "rejected" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 0, "stale index rejects replacement card");
        Reset();
        ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3State, Context(), ""));
        RunManager.Instance.State.Player.PlayerCombatState.Hand.Cards[0].Playable = false;
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request()));
        Check(Status(result) == "rejected" && result.Response.Contains("stale_generation", StringComparison.Ordinal), "playability change invalidates generation");
        Reset(); result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request()));
        RunManager.Instance.State.Player.PlayerCombatState.Energy--;
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Operation, Context(), "op"));
        Check(Status(result) == "unknown", "unrelated energy change cannot settle unexecuted queued card action");
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context("retry"), Request(correlation: "retry")));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "same operation/action with new request correlation replays");
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request().Replace("\"card_index\":0", "\"card_index\":1", StringComparison.Ordinal)));
        Check(Status(result) == "rejected" && result.Response.Contains("idempotency_conflict", StringComparison.Ordinal), "different semantic action conflicts");
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request().Replace(",", ", ", StringComparison.Ordinal)));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "JSON formatting does not change operation identity");
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request("next")));
        Check(Status(result) == "rejected" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "unknown operation prevents a second mutation");
        Reset(); RunManager.Instance.ActionQueueSynchronizer.ThrowAfterEnqueue = true;
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request()));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "exception after enqueue remains unknown");
        Reset(); var v2 = new RuntimeContext("instance", "caller2", "session2", "lease2", "2", "corr");
        Check(TryAuthorizeRuntimeV2Context(v2, out _) && !TryAuthorizeRuntimeV3GameplayContext(Context(), out _), "different owner/lease cannot bind v2 and v3 against one host");
        Reset();
        Check(TryAuthorizeRuntimeV3GameplayContext(Context(), out _) && !TryAuthorizeRuntimeV2Context(v2, out _), "shared identity fence works in reverse order");
        Reset();
        string v2Body = Request().Replace(RuntimeV3GameplayProtocolVersion, RuntimeV2ProtocolVersion, StringComparison.Ordinal)
            .Replace(RuntimeV3GameplaySchemaDigest, RuntimeV2SchemaDigest, StringComparison.Ordinal)
            .Replace("runtime-v3-gameplay", "runtime-v2", StringComparison.Ordinal)
            .Replace("\"action_id\":\"play_card\",\"card_index\":0,\"target_id\":null", "\"action_id\":\"end_turn\"", StringComparison.Ordinal);
        result = ProcessRuntimeV2Work(new(RuntimeRequestKindRuntimeV2Action, Context(), v2Body));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "v2 enqueue remains uncertain");
        RunManager.Instance.State.Player.PlayerCombatState.TurnNumber++;
        result = ProcessRuntimeV2Work(new(RuntimeRequestKindRuntimeV2Operation, Context(), "op"));
        Check(Status(result) == "unknown", "unrelated next turn cannot settle v2 operation");
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request()));
        Check(Status(result) == "rejected" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "v2 pending operation excludes v3 mutation");
        Reset();
        ProcessRuntimeV2Work(new(RuntimeRequestKindRuntimeV2State, Context(), ""));
        RunManager.Instance = new();
        result = ProcessRuntimeV2Work(new(RuntimeRequestKindRuntimeV2Action, Context(), v2Body));
        Check(Status(result) == "rejected" && result.Response.Contains("stale_generation", StringComparison.Ordinal), "v2 new run invalidates old generation despite identical turn number");
        Reset();
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Action, Context(), Request("run/operation")));
        Check(Status(result) == "unknown", "bounded slash operation identity is admitted");
        result = ProcessRuntimeV3GameplayWork(new(RuntimeRequestKindRuntimeV3Operation, Context(), "run/operation"));
        Check(Status(result) == "unknown" && RunManager.Instance.ActionQueueSynchronizer.Queued.Count == 1, "slash operation identity is reconciled without new mutation");
    }
}
