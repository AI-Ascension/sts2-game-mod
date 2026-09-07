// SPDX-License-Identifier: MIT

using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    public CoopNativeDispatchResult SubmitSharedVote(CoopSharedVoteRequest request)
    {
        if (!TryGetLocalPlayer(request.VoterPeerId, out RunManager manager,
                out INetGameService service, out Player player, out string error))
        {
            return CoopNativeDispatchResult.Rejected(error);
        }

        if (!NativeVoteEncoding.TryParse(request, out NativeVoteEncoding vote, out error))
        {
            return CoopNativeDispatchResult.Rejected(error);
        }

        if (_authorityEpoch is null)
            return CoopNativeDispatchResult.Rejected("native_authority_epoch_unavailable");

        ulong[] participantNativeIds = ConnectedNativePeerIds(service, service.NetId);
        if (participantNativeIds.Length < 2)
            return CoopNativeDispatchResult.Rejected("native_peer_roster_unavailable");

        try
        {
            switch (vote.Domain)
            {
                case CoopVoteDomain.SharedEvent:
                    if (manager.EventSynchronizer is null)
                        return CoopNativeDispatchResult.Rejected("native_event_synchronizer_unavailable");
                    EventModel? eventModel;
                    try
                    {
                        eventModel = manager.EventSynchronizer.GetEventForPlayer(player);
                    }
                    catch
                    {
                        return CoopNativeDispatchResult.Rejected("native_event_state_unavailable");
                    }
                    if (eventModel is null || eventModel.IsFinished
                        || vote.Index >= eventModel.CurrentOptions.Count)
                    {
                        return CoopNativeDispatchResult.Rejected("native_event_choice_out_of_range");
                    }
                    if (!CanRetainPending(request.OperationId)) return CoopNativeDispatchResult.Rejected("native_pending_capacity_exhausted");
                    _pending[request.OperationId] = NativePendingOperation.ForEvent(
                        request.OperationId, request.Domain, _hostSequence, _nativeChecksumOrdinal,
                        player, vote.Index, manager.EventSynchronizer, _authorityEpoch,
                        participantNativeIds);
                    // ChooseLocalOption sends the native option message; the EventSynchronizer
                    // consumer records it in GetPlayerVote on this game thread.
                    try
                    {
                        manager.EventSynchronizer.ChooseLocalOption(vote.Index);
                    }
                    catch
                    {
                        return CoopNativeDispatchResult.Unknown("native_vote_dispatch_outcome_unknown");
                    }
                    return CoopNativeDispatchResult.Accepted();

                case CoopVoteDomain.TreasureRelic:
                    if (manager.TreasureRoomRelicSynchronizer is null)
                        return CoopNativeDispatchResult.Rejected("native_relic_synchronizer_unavailable");
                    var currentRelics = manager.TreasureRoomRelicSynchronizer.CurrentRelics;
                    if (!vote.SkipRelic
                        && (currentRelics is null || vote.Index >= currentRelics.Count))
                    {
                        return CoopNativeDispatchResult.Rejected("native_relic_choice_out_of_range");
                    }
                    if (!CanRetainPending(request.OperationId)) return CoopNativeDispatchResult.Rejected("native_pending_capacity_exhausted");
                    _pending[request.OperationId] = NativePendingOperation.ForRelic(
                        request.OperationId, request.Domain, _hostSequence, _nativeChecksumOrdinal,
                        player, () => _nativeChecksumOrdinal, _authorityEpoch,
                        participantNativeIds);
                    if (vote.SkipRelic)
                    {
                        try
                        {
                            manager.TreasureRoomRelicSynchronizer.SkipRelicLocally();
                        }
                        catch
                        {
                            return CoopNativeDispatchResult.Unknown("native_vote_dispatch_outcome_unknown");
                        }
                    }
                    else
                    {
                        try
                        {
                            manager.TreasureRoomRelicSynchronizer.PickRelicLocally(vote.Index);
                        }
                        catch
                        {
                            return CoopNativeDispatchResult.Unknown("native_vote_dispatch_outcome_unknown");
                        }
                    }
                    return CoopNativeDispatchResult.Accepted();

                default:
                    return CoopNativeDispatchResult.Rejected("native_vote_domain_not_supported");
            }
        }
        catch
        {
            return CoopNativeDispatchResult.Unknown("native_vote_dispatch_outcome_unknown");
        }
    }
}
