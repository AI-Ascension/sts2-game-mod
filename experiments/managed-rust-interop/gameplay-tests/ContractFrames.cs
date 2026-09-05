// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

// Deterministic production-handler output for cross-language consumer validation.
// Synthetic host only: no network, game, profile, save or provider dependency.
internal static class ContractFrames
{
    internal static void Emit()
    {
        var source = new FakeHost();
        RuntimeV3GameplaySupport support = RuntimeV3GameplaySupport.WithHost(source, new TestQueue());
        foreach (string kind in new[] { "state_request", "reobserve_request", "legal_actions_request", "recover_request" })
        {
            Emit(Wire.Call(support, kind, 1, out _));
        }
        Emit(Wire.Call(support, "dispatch_action_request", 1, out _));
        Emit(Wire.Call(support, "wait_request", 1, out _));
        Emit(Wire.Call(support, "recover_request", 1, out _, recoveryKind: "reconcile"));
        source.Complete = true;
        Emit(Wire.Call(support, "recover_request", 1, out _, recoveryKind: "reconcile"));
        Emit(Wire.Call(support, "wait_request", 1, out _));
        Emit(Wire.Call(support, "dispatch_action_request", 1, out _));
        Emit(Wire.Call(support, "dispatch_action_request", 1, out _, stateId: "conflict"));
        Emit(Wire.Call(support, "recover_request", 1, out _, session: "foreign", recoveryKind: "reconcile"));
        var queue = new TestQueue { Deferred = true };
        var rejectedSource = new FakeHost();
        RuntimeV3GameplaySupport rejected = RuntimeV3GameplaySupport.WithHost(rejectedSource, queue);
        Emit(Wire.Call(rejected, "dispatch_action_request", 1, out _));
        rejectedSource.Generation = 2;
        queue.Run();
        Emit(Wire.Call(rejected, "wait_request", 1, out _));
        Emit(Wire.Call(rejected, "recover_request", 1, out _, recoveryKind: "reconcile"));
        RuntimeV3GameplaySupport stale = RuntimeV3GameplaySupport.WithHost(new FakeHost { Generation = 2 }, new TestQueue());
        Emit(Wire.Call(stale, "dispatch_action_request", 1, out _));
        Emit(Wire.Call(RuntimeV3GameplaySupport.Unconfigured(), "dispatch_action_request", 1, out _));
        Emit(Wire.Call(RuntimeV3GameplaySupport.Unconfigured(), "recover_request", 1, out _));
    }

    private static void Emit(JsonDocument document)
    {
        using (document) { Console.WriteLine(document.RootElement.GetRawText()); }
    }
}
