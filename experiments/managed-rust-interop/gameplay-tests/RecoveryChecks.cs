// SPDX-License-Identifier: MIT

using System;
using System.IO;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class RecoveryChecks
{
    internal static void Run()
    {
        CanonicalVectorsMatchProtocolArtifact();
        DurableAdmissionRetainsMoreThanSmallCaps();
        FenceReplacementRejectsQueuedWork();
        UnknownAndHistoricalReadSurviveReopen();
        CorruptJournalBlocksAdmission();
    }

    private static void CanonicalVectorsMatchProtocolArtifact()
    {
        (LegalActionReference Action, string Json, string Digest)[] vectors =
        {
            (new("action-end-turn", "end_turn", null, null, 0),
                "{\"action\":{\"kind\":\"end_turn\"},\"action_id\":\"action-end-turn\"}",
                "4fd3e287a56eda15d4454d4b15a576110db374d68eecb1a82445ce4a9ea21159"),
            (new("action-start-run", "start_run", "ironclad", null, 0),
                "{\"action\":{\"character_id\":\"ironclad\",\"kind\":\"start_run\"},\"action_id\":\"action-start-run\"}",
                "6464c6d2571ce0c697b147c1554625e1128d730f131969b52c5c2d85ccea6c1c"),
            (new("action-select-map-node", "select_map_node", "node-1", null, 0),
                "{\"action\":{\"kind\":\"select_map_node\",\"node_id\":\"node-1\"},\"action_id\":\"action-select-map-node\"}",
                "02877d732323ad614fbf9f376626b65a33d88856af5e96a7b552ab8da9b55781"),
            (new("action-play-card", "play_card", "strike", null, 0),
                "{\"action\":{\"card_id\":\"strike\",\"kind\":\"play_card\",\"target_id\":null},\"action_id\":\"action-play-card\"}",
                "d4abf283c21ca197d6d90257731d47e1d233da9a98035f8414ddda21c33328c2"),
            (new("action-choose-reward", "choose_reward", "reward-1", null, 0),
                "{\"action\":{\"kind\":\"choose_reward\",\"reward_id\":\"reward-1\"},\"action_id\":\"action-choose-reward\"}",
                "156456e58713a140576919e95035938c2f15639fc2e171e750a5726057c6f4fd"),
            (new("action-shop-purchase", "shop_purchase", "item-1", null, 0),
                "{\"action\":{\"item_id\":\"item-1\",\"kind\":\"shop_purchase\"},\"action_id\":\"action-shop-purchase\"}",
                "c0f408b94538f2749e1af046487af84d5e9d56b071429cb9849adbcee4c06220"),
            (new("action-shop-remove", "shop_remove", "card-1", null, 0),
                "{\"action\":{\"card_id\":\"card-1\",\"kind\":\"shop_remove\"},\"action_id\":\"action-shop-remove\"}",
                "1204f8dcc9e1618d39d40b64040827be208717008ddb2b241a39c666c58ef3e5"),
            (new("action-event-choice", "event_choice", "choice-1", null, 0),
                "{\"action\":{\"choice_id\":\"choice-1\",\"kind\":\"event_choice\"},\"action_id\":\"action-event-choice\"}",
                "680994e7a534d3b24090dbd825d25aa9269eb6417c3351ac00900b77391c1b9e")
        };
        foreach ((LegalActionReference action, string json, string digest) in vectors)
        {
            Check(RuntimeV3GameplayRecoveryCanonical.TryCreate(action, out string actualJson,
                    out string actualDigest)
                && actualJson == json && actualDigest == digest,
                "RCJ-1 vector matches protocol artifact");
            Check(RuntimeV3GameplayRecoveryCanonical.TryRead(json, digest, 0,
                    out LegalActionReference? restored) && restored == action,
                "RCJ-1 vector round-trips through strict parser");
        }
        string digestForMalformed = vectors[0].Digest;
        foreach (string malformed in new[]
        {
            "{\"action\":{\"kind\":\"end_turn\"},\"action\":{\"kind\":\"end_turn\"},\"action_id\":\"action-end-turn\"}",
            "{\"action\":{\"kind\":\"end_turn\"},\"action_id\":\"é\"}",
            "{\"action\":{\"kind\":\"end_turn\"},\"action_id\":1.0}",
            "{\"action\":{\"kind\":\"end_turn\"},\"action_id\":\"action\\u002dend-turn\"}"
        })
        {
            Check(!RuntimeV3GameplayRecoveryCanonical.TryRead(malformed, digestForMalformed, 0,
                out _), "RCJ-1 malformed vector is rejected");
        }
    }

    private static void DurableAdmissionRetainsMoreThanSmallCaps()
    {
        string directory = NewDirectory();
        try
        {
            RuntimeV3RecoveryCredentials credentials = Credentials();
            using var store = new RuntimeV3GameplayRecoveryStore(directory, credentials);
            RuntimeV3RecoveryRelease release = Release();
            RuntimeV3HostFence fence = Fence(1, 1, 1);
            Check(store.TryReplaceFence(Bootstrap(fence, release, credentials), out _),
                "authenticated bootstrap must install the first fence");
            var source = new FakeHost { Complete = false };
            var queue = new TestQueue { Deferred = true };
            var host = new RuntimeV3GameplayHost(source, queue, null, store);
            RuntimeV3GameplayObservation observation = source.Observe();
            LegalActionReference action = RuntimeV3GameplayFixtures.EndTurn(observation.Generation);
            for (int index = 0; index < 96; index++)
            {
                RuntimeV3OperationKey operation = Operation(index, fence);
                RuntimeV3DispatchReceipt receipt = host.Dispatch(operation, observation, action);
                Check(receipt.Status == RuntimeV3DispatchStatus.Accepted,
                    "durable admission must remain available beyond 64 operations");
            }
            Check(store.OperationCount == 96 && queue.PendingCount == 96,
                "durable retention and bounded queue must account for every unresolved operation");
            RuntimeV3DispatchReceipt replay = host.Dispatch(Operation(95, fence), observation, action);
            Check(replay.Status == RuntimeV3DispatchStatus.Accepted && queue.PendingCount == 96,
                "exact retry must replay without queueing a second mutation");
        }
        finally
        {
            Directory.Delete(directory, true);
        }
    }

    private static void FenceReplacementRejectsQueuedWork()
    {
        string directory = NewDirectory();
        try
        {
            RuntimeV3RecoveryCredentials credentials = Credentials();
            using var store = new RuntimeV3GameplayRecoveryStore(directory, credentials);
            RuntimeV3RecoveryRelease release = Release();
            RuntimeV3HostFence first = Fence(1, 1, 1);
            Check(store.TryReplaceFence(Bootstrap(first, release, credentials), out _),
                "first fence bootstrap");
            var source = new FakeHost();
            var queue = new TestQueue { Deferred = true };
            var host = new RuntimeV3GameplayHost(source, queue, null, store);
            RuntimeV3GameplayObservation observation = source.Observe();
            LegalActionReference action = RuntimeV3GameplayFixtures.EndTurn(observation.Generation);
            RuntimeV3OperationKey operation = Operation(101, first);
            Check(host.Dispatch(operation, observation, action).Status == RuntimeV3DispatchStatus.Accepted,
                "queued operation admission");
            RuntimeV3HostFence replacement = Fence(2, 2, 2);
            Check(store.TryReplaceFence(Bootstrap(replacement, release, credentials), out _),
                "replacement bootstrap");
            queue.Run();
            Check(source.Dispatches == 0, "stale queued work must not reach the host effect");
            Check(host.TryGetReceipt(operation, out RuntimeV3DispatchReceipt? receipt)
                && receipt?.Status == RuntimeV3DispatchStatus.Rejected,
                "stale queued work must have a terminal rejection");
            Check(store.TryGet(operation, out RuntimeV3HostOperationRecord? durable)
                && durable?.ErrorCode is ("stale_fence" or "authority_rotated"),
                "durable fence rejection must retain the reason");
        }
        finally
        {
            Directory.Delete(directory, true);
        }
    }

    private static void UnknownAndHistoricalReadSurviveReopen()
    {
        string directory = NewDirectory();
        RuntimeV3OperationKey operation;
        string digest;
        try
        {
            RuntimeV3RecoveryCredentials credentials = Credentials();
            RuntimeV3RecoveryRelease release = Release();
            RuntimeV3HostFence fence = Fence(3, 3, 3);
            operation = Operation(202, fence);
            using (var store = new RuntimeV3GameplayRecoveryStore(directory, credentials))
            {
                Check(store.TryReplaceFence(Bootstrap(fence, release, credentials), out _),
                    "unknown test bootstrap");
                var source = new FakeHost();
                var host = new RuntimeV3GameplayHost(source, new TestQueue(), null, store);
                RuntimeV3GameplayObservation observation = source.Observe();
                LegalActionReference action = RuntimeV3GameplayFixtures.EndTurn(observation.Generation);
                Check(host.Dispatch(operation, observation, action).Status == RuntimeV3DispatchStatus.Unknown,
                    "missing operation completion must remain unknown");
                Check(RuntimeV3GameplayRecoveryCanonical.TryCreate(action, out _, out digest),
                    "canonical action digest");
                Check(store.TryGet(operation, out RuntimeV3HostOperationRecord? record)
                    && record?.Status == RuntimeV3DispatchStatus.Unknown
                    && record.Witness is null,
                    "unknown receipt must be durably retained without a fabricated witness");
                string proof = RuntimeV3GameplayRecoveryContract.CreateHistoricalReadProof(
                    credentials.HistoricalReadSecret, operation, digest);
                Check(store.TryHistoricalLookup(operation, digest, proof,
                    out RuntimeV3HistoricalOperation? historical, out _)
                    && historical?.MutationAuthorized == false,
                    "historical recovery reads can never authorize mutation");
            }

            Check(RuntimeV3GameplayRecoveryStore.TryOpen(
                directory, credentials, out RuntimeV3GameplayRecoveryStore? reopened, out _)
                && reopened is not null, "journal must reopen");
            using (reopened!)
            {
                var source = new FakeHost { Complete = true };
                var host = new RuntimeV3GameplayHost(source, new TestQueue(), null, reopened);
                RuntimeV3GameplayObservation observation = source.Observe();
                LegalActionReference action = RuntimeV3GameplayFixtures.EndTurn(observation.Generation);
                Check(host.TryReplay(operation, observation.StateId, action,
                    out RuntimeV3DispatchReceipt? replay) && replay?.Status == RuntimeV3DispatchStatus.Unknown,
                    "reopened unknown operation must replay conservatively");
                Check(source.Dispatches == 0, "recovery lookup must not resend the mutation");
            }
        }
        finally
        {
            Directory.Delete(directory, true);
        }
    }

    private static void CorruptJournalBlocksAdmission()
    {
        string directory = NewDirectory();
        try
        {
            RuntimeV3RecoveryCredentials credentials = Credentials();
            using (var store = new RuntimeV3GameplayRecoveryStore(directory, credentials))
            {
                Check(store.IsHealthy, "fresh synthetic store should be healthy");
            }
            File.AppendAllText(Path.Combine(directory, "runtime-v3-recovery-v1.journal"), "torn");
            Check(!RuntimeV3GameplayRecoveryStore.TryOpen(
                directory, credentials, out _, out string error)
                && error == "persistence_unavailable",
                "torn journal must block admission rather than reset state");
        }
        finally
        {
            Directory.Delete(directory, true);
        }
    }

    private static RuntimeV3RecoveryCredentials Credentials() => new(
        Bytes(11),
        Bytes(22),
        Bytes(33));

    private static byte[] Bytes(byte value)
    {
        var bytes = new byte[32];
        Array.Fill(bytes, value);
        return bytes;
    }

    private static RuntimeV3RecoveryRelease Release() => new(
        new string('a', 64),
        new string('b', 64),
        new string('c', 64),
        RuntimeV3GameplayContract.SchemaDigest);

    private static RuntimeV3HostFence Fence(int generation, int fenceGeneration, int leaseEpoch) =>
        new(
            Guid.NewGuid().ToString("D"),
            Guid.NewGuid().ToString("D"),
            Guid.NewGuid().ToString("D"),
            Guid.NewGuid().ToString("D"),
            (ulong)generation,
            Guid.NewGuid().ToString("D"),
            (ulong)leaseEpoch,
            Guid.NewGuid().ToString("D"),
            (ulong)fenceGeneration,
            DateTimeOffset.UtcNow.AddMinutes(5));

    private static RuntimeV3HostBootstrapRequest Bootstrap(
        RuntimeV3HostFence fence,
        RuntimeV3RecoveryRelease release,
        RuntimeV3RecoveryCredentials credentials) =>
        new(
            fence,
            release,
            RuntimeV3GameplayRecoveryContract.Contract,
            RuntimeV3GameplayRecoveryContract.SchemaDigest,
            RuntimeV3GameplayRecoveryContract.CreateBootstrapProof(
                credentials.BootstrapSecret, fence, release));

    private static RuntimeV3OperationKey Operation(int index, RuntimeV3HostFence fence) => new(
        fence.InstanceId,
        "00000000-0000-4000-8000-000000000001",
        fence.LeaseId,
        fence.LeaseEpoch,
        $"00000000-0000-4000-8000-{index:X12}".ToLowerInvariant());

    private static string NewDirectory()
    {
        string path = Path.Combine(Path.GetTempPath(), "sts2-recovery-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(path);
        return path;
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }
    }
}
