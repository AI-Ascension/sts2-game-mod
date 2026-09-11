// SPDX-License-Identifier: MIT

using System;
using Godot;
using MegaCrit.Sts2.Core.Multiplayer.Game;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // Runtime-v2 seeded-run owns callback IDs 9 and 10. Keep co-op callbacks in a range that is
    // disjoint from every existing runtime profile in the seeded mainline.
    private const uint RuntimeRequestKindCoopObservation = 16;
    private const uint RuntimeRequestKindCoopAction = 17;
    private const uint RuntimeRequestKindCoopVote = 18;
    private const uint RuntimeRequestKindCoopRejoin = 19;
    private const uint RuntimeRequestKindCoopRecover = 20;
    private const uint RuntimeRequestKindCoopLegalCatalog = 21;
    private static CoopNativeRuntime? _coopNativeRuntime;
    private static CoopOpaqueOperationHostDispatcher? _opaqueOperationHostDispatcher;
    private static CoopOpaqueOperationClientBridge? _opaqueOperationClientBridge;
    private static SceneTree? _opaqueOperationPumpTree;
    private static Action? _opaqueOperationPump;

    /// <summary>
    /// True while a co-op action or vote has been accepted by the host adapter but its native
    /// effect is not yet settled. The gameplay profiles use this predicate to serialize their
    /// own mutations against the co-op profile.
    /// </summary>
    internal static bool HasPendingCoopMutation => _coopNativeRuntime?.HasPendingMutation ?? false;

    /// <summary>
    /// Installs the managed host port; the caller must provide a game-thread port. The optional
    /// predicate must return true only when the other mutation profiles are idle. It is checked
    /// immediately before each native co-op mutation, after a fresh co-op observation.
    /// </summary>
    internal static void ConfigureCoopNative(ICoopNativeHostPort port, Func<bool>? canDispatch = null)
    {
        _coopNativeRuntime = new CoopNativeRuntime(port,
            canDispatch ?? CanDispatchCoopNativeMutation);
        _opaqueOperationHostDispatcher = new CoopOpaqueOperationHostDispatcher(_coopNativeRuntime);
        _opaqueOperationClientBridge = new CoopOpaqueOperationClientBridge(
            () => _opaqueOperationAdapter);
        ConfigureOpaqueOperationNativeTransport(_opaqueOperationHostDispatcher, IsAuthenticatedNativeHost,
            _opaqueOperationClientBridge.ReceiveAuthenticatedHostReply);
        InstallOpaqueOperationGameThreadPump();
        NativeCoopSessionController.StartIfConfigured();
#if STS2_NATIVE_COOP_PROBE
        CoopNativeLobbyProbe.StartIfEnabled(port);
#endif
    }

    private static bool CanDispatchCoopNativeMutation() =>
        !HasPendingNonCoopMutation() && !SeededRunStandardHost.HasPendingMutation;

    private static bool IsAuthenticatedNativeHost(ulong senderId)
    {
        if (NativeCoopSessionController.ActiveService is not INetClientGameService client
            || !client.IsConnected || client.NetClient is not { IsConnected: true } nativeClient)
        {
            return false;
        }

        return nativeClient.HostNetId != 0 && nativeClient.HostNetId == senderId;
    }

    private static void InstallOpaqueOperationGameThreadPump()
    {
        if (Engine.GetMainLoop() is not SceneTree tree)
            return;
        if (ReferenceEquals(_opaqueOperationPumpTree, tree))
            return;
        if (_opaqueOperationPumpTree is not null && _opaqueOperationPump is not null)
            _opaqueOperationPumpTree.ProcessFrame -= _opaqueOperationPump;

        _opaqueOperationPumpTree = tree;
        _opaqueOperationPump = () =>
        {
            _opaqueOperationHostDispatcher?.Pump();
            _opaqueOperationAdapter?.Pump();
        };
        tree.ProcessFrame += _opaqueOperationPump;
    }

    /// <summary>Explicit route hook for RuntimeInterop's native callback dispatcher.</summary>
    private static (int Status, string Response) ProcessCoopNativeWork(
        uint kind, RuntimeContext context, string body)
    {
        CoopNativeRuntime? runtime = _coopNativeRuntime;
        if (runtime is null)
        {
            return (503, "{\"error_code\":\"coop_native_unconfigured\"}");
        }
        string expected = kind switch
        {
            RuntimeRequestKindCoopObservation => "observation",
            RuntimeRequestKindCoopAction => "local_action_request",
            RuntimeRequestKindCoopVote => "shared_vote_request",
            RuntimeRequestKindCoopRejoin => "rejoin_request",
            RuntimeRequestKindCoopRecover => "recovery_response",
            RuntimeRequestKindCoopLegalCatalog => "legal_catalog_request",
            _ => string.Empty
        };
        return expected.Length == 0
            ? (400, "{\"error_code\":\"coop_native_unknown_route\"}")
            : ProcessCoopNativeClientOrHost(runtime, expected, context, body);
    }

    private static (int Status, string Response) ProcessCoopNativeClientOrHost(
        CoopNativeRuntime runtime, string expected, RuntimeContext context, string body)
    {
        if (NativeCoopSessionController.ActiveService is not INetClientGameService)
        {
            return runtime.Handle(context.InstanceId, context.SessionId, context.LeaseId,
                context.CorrelationId, context.LeaseEpoch, expected, body);
        }
        if (_opaqueOperationClientBridge is null)
            return (503, "{\"error_code\":\"coop_native_client_transport_unavailable\"}");

        if (expected is "local_action_request" or "shared_vote_request")
        {
            if (!_opaqueOperationClientBridge.TrySubmit(runtime, expected, context.InstanceId,
                    context.SessionId, context.LeaseId, context.CorrelationId, context.LeaseEpoch,
                    body, out _, out string errorCode))
            {
                return (409, "{\"error_code\":\"" + errorCode + "\"}");
            }
            return runtime.ClientCarrierReceipt(context.InstanceId, context.SessionId,
                context.LeaseId, context.CorrelationId, context.LeaseEpoch, body, null);
        }
        if (expected == "recovery_response")
        {
            _opaqueOperationClientBridge.TryReconcile(out OpaqueOperationNativeMessage? reply,
                out string originalBody);
            return originalBody.Length == 0
                ? (409, "{\"error_code\":\"coop_native_reconcile_unknown_operation\"}")
                : runtime.ClientCarrierReceipt(context.InstanceId, context.SessionId,
                    context.LeaseId, context.CorrelationId, context.LeaseEpoch, originalBody, reply);
        }
        // Rejoin owns a local client's native connection lifecycle. It must use the existing
        // local rejoin path, which creates and observes that client connection; a remote opaque
        // carrier cannot safely create a rejoin on another process's behalf.
        if (expected == "rejoin_request")
        {
            return runtime.Handle(context.InstanceId, context.SessionId, context.LeaseId,
                context.CorrelationId, context.LeaseEpoch, expected, body);
        }
        return (409, "{\"error_code\":\"coop_native_client_route_not_supported\"}");
    }
}
