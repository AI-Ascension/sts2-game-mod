// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Map;
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

        // ChecksumDataMessage is client-to-host in the installed native service. A client can
        // emit its local vote, but this port has no host receipt proxy with which to correlate
        // that vote to the authoritative host checkpoint. Do not return an accepted pending
        // operation that can never settle locally; the harness must submit the authoritative
        // operation through the native host route and coordinate client votes separately.
        if (service.Type != NetGameType.Host)
            return CoopNativeDispatchResult.Rejected("native_vote_requires_host_authority");

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
                    bool eventIsShared;
                    try
                    {
                        eventIsShared = manager.EventSynchronizer.IsShared;
                    }
                    catch
                    {
                        return CoopNativeDispatchResult.Rejected("native_event_state_unavailable");
                    }
                    if (!eventIsShared)
                        return CoopNativeDispatchResult.Rejected("native_event_is_not_shared");
                    EventModel? eventModel;
                    try
                    {
                        eventModel = manager.EventSynchronizer.GetEventForPlayer(player);
                    }
                    catch
                    {
                        return CoopNativeDispatchResult.Rejected("native_event_state_unavailable");
                    }
                    if (eventModel is null || eventModel.IsFinished || vote.Index < 0
                        || vote.Index >= eventModel.CurrentOptions.Count
                        || eventModel.CurrentOptions[vote.Index].IsLocked)
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
                    if (currentRelics is null)
                        return CoopNativeDispatchResult.Rejected("native_relic_picking_unavailable");
                    TreasureRoomRelicSynchronizer.PlayerVote currentVote;
                    try
                    {
                        currentVote = manager.TreasureRoomRelicSynchronizer.GetPlayerVote(player);
                    }
                    catch
                    {
                        return CoopNativeDispatchResult.Rejected("native_relic_vote_unavailable");
                    }
                    if (currentVote.voteReceived)
                        return CoopNativeDispatchResult.Rejected("native_relic_vote_already_received");
                    if (!vote.SkipRelic
                        && (vote.Index < 0 || vote.Index >= currentRelics.Count))
                    {
                        return CoopNativeDispatchResult.Rejected("native_relic_choice_out_of_range");
                    }
                    if (!CanRetainPending(request.OperationId)) return CoopNativeDispatchResult.Rejected("native_pending_capacity_exhausted");
                    _pending[request.OperationId] = NativePendingOperation.ForRelic(
                        request.OperationId, request.Domain, _hostSequence, _nativeChecksumOrdinal,
                        player, manager.TreasureRoomRelicSynchronizer, currentRelics,
                        vote.Index, vote.SkipRelic, _authorityEpoch,
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

                case CoopVoteDomain.Map:
                    if (manager.MapSelectionSynchronizer is null)
                        return CoopNativeDispatchResult.Rejected("native_map_synchronizer_unavailable");
                    if (manager.DebugOnlyGetState() is not { } mapRun
                        || vote.MapRow < 0
                        || vote.MapColumn < 0)
                    {
                        return CoopNativeDispatchResult.Rejected("native_map_state_unavailable");
                    }

                    var destination = new MapCoord(vote.MapColumn, vote.MapRow);

                    int mapGenerationCount = vote.MapGenerationCount < 0
                        ? manager.MapSelectionSynchronizer.MapGenerationCount
                        : vote.MapGenerationCount;
                    if (mapGenerationCount != manager.MapSelectionSynchronizer.MapGenerationCount)
                        return CoopNativeDispatchResult.Rejected("native_map_vote_stale_generation");
                    if (vote.MapActIndex is { } mapAct
                        && mapAct != mapRun.CurrentActIndex)
                    {
                        return CoopNativeDispatchResult.Rejected("native_map_vote_wrong_act");
                    }
                    if (mapRun.Map is null || !mapRun.Map.HasPoint(destination))
                        return CoopNativeDispatchResult.Rejected("native_map_coordinate_unavailable");
                    MapPoint? sourcePoint = mapRun.CurrentMapPoint;
                    if (sourcePoint is null
                        || !MapTravel.GetTravelablePointsFrom(mapRun, sourcePoint)
                            .Any(point => point.coord == destination))
                    {
                        return CoopNativeDispatchResult.Rejected("native_map_destination_not_reachable");
                    }

                    var mapVote = new MapVote
                    {
                        mapGenerationCount = mapGenerationCount,
                        coord = destination
                    };
                    if (!CanRetainPending(request.OperationId)) return CoopNativeDispatchResult.Rejected("native_pending_capacity_exhausted");
                    _pending[request.OperationId] = NativePendingOperation.ForMap(
                        request.OperationId, request.Domain, _hostSequence, _nativeChecksumOrdinal,
                        player, manager.MapSelectionSynchronizer, mapVote, mapRun,
                        mapRun.MapLocation, _authorityEpoch,
                        participantNativeIds);
                    try
                    {
                        // PlayerVotedForMapCoord is the first-party local vote path. It emits
                        // the native map message; the synchronizer's host-side tally performs
                        // the eventual shared move once all authenticated peers have voted.
                        manager.MapSelectionSynchronizer.PlayerVotedForMapCoord(
                            player, mapRun.MapLocation, mapVote);
                    }
                    catch
                    {
                        return CoopNativeDispatchResult.Unknown("native_vote_dispatch_outcome_unknown");
                    }
                    return CoopNativeDispatchResult.Accepted();

                case CoopVoteDomain.PlayerChoice:
                    if (vote.PlayerChoice is not { } playerChoice)
                        return CoopNativeDispatchResult.Rejected("native_player_choice_encoding_invalid");
                    return SubmitPlayerChoiceVote(
                        manager, player, request, playerChoice, participantNativeIds);

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
