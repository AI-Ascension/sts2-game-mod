// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private CoopNativeDispatchResult SubmitPlayerChoiceVote(
        RunManager manager,
        Player player,
        CoopSharedVoteRequest request,
        NativePlayerChoiceEncoding playerChoice,
        ulong[] participantNativeIds)
    {
        if (manager.PlayerChoiceSynchronizer is null)
            return CoopNativeDispatchResult.Rejected("native_player_choice_synchronizer_unavailable");

        // The runtime's admission observation includes the visible choice catalog fingerprint.
        // Re-read it immediately before touching the UI so a stale index cannot target a later
        // selector or a different virtualized row window.
        CoopHostObservation current = Observe();
        if (current.HostGeneration != request.ExpectedHostGeneration)
            return CoopNativeDispatchResult.Rejected("native_player_choice_stale_generation");

        GameAction? pausedAction = manager.ActionExecutor?.CurrentlyRunningAction;
        if (pausedAction is null
            || pausedAction.State != GameActionState.GatheringPlayerChoice
            || pausedAction.OwnerId != player.NetId)
        {
            return CoopNativeDispatchResult.Rejected("native_player_choice_action_not_paused");
        }

        uint choiceId;
        try
        {
            RunState choiceRun = manager.DebugOnlyGetState()
                ?? throw new InvalidOperationException("run state unavailable");
            int slot = choiceRun.GetPlayerSlotIndex(player);
            if (slot < 0 || slot >= manager.PlayerChoiceSynchronizer.ChoiceIds.Count)
                return CoopNativeDispatchResult.Rejected("native_player_choice_unavailable");
            uint nextChoiceId = manager.PlayerChoiceSynchronizer.ChoiceIds[slot];
            if (nextChoiceId == 0)
                return CoopNativeDispatchResult.Rejected("native_player_choice_unavailable");
            // ChoiceIds is the next reservation counter, while the paused action owns the most
            // recently reserved ID.
            choiceId = nextChoiceId - 1;
        }
        catch
        {
            return CoopNativeDispatchResult.Rejected("native_player_choice_unavailable");
        }

        PlayerChoiceResult result;
        try
        {
            result = playerChoice.Kind switch
            {
                NativePlayerChoiceKind.Index =>
                    PlayerChoiceResult.FromIndex(playerChoice.Index),
                NativePlayerChoiceKind.Indexes =>
                    PlayerChoiceResult.FromIndexes(playerChoice.Indexes!.ToList()),
                NativePlayerChoiceKind.Player =>
                    PlayerChoiceResult.FromPlayerId(playerChoice.PlayerId),
                _ => throw new InvalidOperationException("unknown player choice kind")
            };
        }
        catch
        {
            return CoopNativeDispatchResult.Rejected("native_player_choice_result_invalid");
        }

        if (!IsNativePlayerChoiceResultLegal(manager.DebugOnlyGetState(), player, result))
            return CoopNativeDispatchResult.Rejected("native_player_choice_result_not_legal");

        if (!CanRetainPending(request.OperationId))
            return CoopNativeDispatchResult.Rejected("native_pending_capacity_exhausted");
        _pending[request.OperationId] = NativePendingOperation.ForPlayerChoice(
            request.OperationId, request.Domain, _hostSequence, _nativeChecksumOrdinal,
            player, manager.PlayerChoiceSynchronizer, choiceId, result, _authorityEpoch!,
            participantNativeIds);
        try
        {
            // The local first-party command is waiting on its selection screen. Calling
            // SyncLocalChoice alone only broadcasts a packet and leaves that command's UI task
            // paused forever; complete the supported visible UI surface so the game itself
            // produces the local choice message and resumes the action.
            if (!TryDispatchPlayerChoiceThroughUi(result, out string uiError))
            {
                if (uiError == "native_player_choice_dispatch_outcome_unknown")
                    return CoopNativeDispatchResult.Unknown(uiError);
                _pending.Remove(request.OperationId);
                return CoopNativeDispatchResult.Rejected(uiError);
            }
        }
        catch
        {
            // A signal may have completed the UI before a downstream native callback throws.
            // Keep the pending operation and reconcile it from the first-party choice/checksum
            // callbacks instead of claiming rejection.
            return CoopNativeDispatchResult.Unknown("native_vote_dispatch_outcome_unknown");
        }
        return CoopNativeDispatchResult.Accepted();
    }
}
