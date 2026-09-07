// SPDX-License-Identifier: MIT

using System;
using Godot;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Multiplayer.Transport;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeCoopSessionController
{
    internal static string RejoinStatus { get; private set; } = "idle";

    internal static bool IsRejoinInProgress =>
        _state is { IsRejoin: true, Completed: false };

    internal static bool IsRejoinRecovered =>
        _state is { IsRejoin: true, Completed: true }
        && string.Equals(Status, "client_native_rejoin_recovered", StringComparison.Ordinal)
        && string.Equals(RejoinStatus, "recovered", StringComparison.Ordinal)
        && _state.ClientService is { IsConnected: true };

    /// <summary>
    /// Starts one first-party JoinFlow against the configured ENet host. This method only admits
    /// the bound local client identity; the asynchronous flow and saved-run restore settle on the
    /// normal ProcessFrame callback. A repeated operation is accepted while that same flow is in
    /// progress so the host runtime can preserve idempotency without issuing another connection.
    /// </summary>
    internal static CoopNativeDispatchResult RequestRejoin(
        string opaquePeerId, ulong nativePeerId, ulong rejoinEpoch)
    {
        if (!CoopPeerIdentity.IsOpaque(opaquePeerId))
            return CoopNativeDispatchResult.Rejected("unknown_or_stale_peer_identity");
        if (rejoinEpoch > RuntimeV3GameplayContract.MaxGeneration)
            return CoopNativeDispatchResult.Rejected("invalid_rejoin_epoch");
        ControllerState? state = _state;
        if (state is null || !string.Equals(state.Role, "client", StringComparison.Ordinal))
            return CoopNativeDispatchResult.Rejected("native_rejoin_transport_unconfigured");
        if (state.ClientId == 0 || nativePeerId != state.ClientId)
            return CoopNativeDispatchResult.Rejected("unknown_or_stale_peer_identity");
        if (state.IsRejoin && !state.Completed)
            return CoopNativeDispatchResult.Accepted();
        if (IsRejoinRecovered)
            return CoopNativeDispatchResult.Accepted();
        if (Engine.GetMainLoop() is not SceneTree tree)
            return CoopNativeDispatchResult.Rejected("native_rejoin_scene_tree_unavailable");

        try
        {
            state.JoinFlow?.CancelToken.Cancel();
            if (state.ClientService is { IsConnected: true } oldService)
                oldService.Disconnect(NetError.CancelledJoin, true);
            state.Initializer?.Dispose();
            state.Initializer = null;
            state.JoinFlow = null;
            state.JoinTask = null;
            state.RestoreTask = null;
            state.RejoinResponse = null;
            state.JoinResultHandled = false;
            state.IsRejoin = true;
            state.Completed = false;
            RejoinStatus = "requested";
            AttachProcessFrame(tree);
            Status = "client_rejoin_requested";
            GD.Print($"[AI-ASCENSION COOP LOBBY] native rejoin requested for local_net_id={nativePeerId}; epoch={rejoinEpoch}");
            return CoopNativeDispatchResult.Accepted();
        }
        catch (Exception exception)
        {
            RejoinStatus = "failed:" + exception.GetType().Name;
            return CoopNativeDispatchResult.Unknown("native_rejoin_start_outcome_unknown");
        }
    }

    private static void AttachProcessFrame(SceneTree tree)
    {
        if (_tree is not null && _processFrame is not null)
            return;
        _tree = tree;
        _processFrame = () => Tick(tree);
        tree.ProcessFrame += _processFrame;
    }
}
