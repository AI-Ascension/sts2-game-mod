// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    public CoopNativeDispatchResult DispatchLocalAction(CoopLocalActionRequest request)
    {
        if (!TryGetBoundPlayer(request.ActorPeerId, out RunManager manager,
                out INetGameService service, out Player player, out string error))
        {
            return CoopNativeDispatchResult.Rejected(error);
        }

        // The installed checksum message is client-to-host and this adapter has no supported
        // client-origin action receipt route. Admit local mutations only on the native host so a
        // client cannot receive an Accepted/Unknown receipt that can never settle.
        if (service.Type != NetGameType.Host)
            return CoopNativeDispatchResult.Rejected("native_action_requires_host_authority");

        if (request.ActionKind is not ("end_turn" or "play_card"))
        {
            return CoopNativeDispatchResult.Rejected("native_action_kind_not_supported");
        }

        // The native action encoder carries combat targets in the card value itself. A peer
        // target token belongs to the higher-level gameplay catalogs and has no native meaning;
        // reject it instead of silently acting on a different target than the caller observed.
        if (request.TargetPeerId is not null)
            return CoopNativeDispatchResult.Rejected("native_action_target_peer_unsupported");

        ActionQueueSynchronizer? synchronizer = manager.ActionQueueSynchronizer;
        if (synchronizer is null)
        {
            return CoopNativeDispatchResult.Rejected("native_action_queue_unavailable");
        }

        if (_authorityEpoch is null)
            return CoopNativeDispatchResult.Rejected("native_authority_epoch_unavailable");

        ulong[] participantNativeIds = ConnectedNativePeerIds(service, service.NetId);
        if (participantNativeIds.Length < 2)
            return CoopNativeDispatchResult.Rejected("native_peer_roster_unavailable");

        GameAction action;
        if (request.ActionKind == "end_turn")
        {
            if (!NativeActionEncoding.TryTurnNumber(request.Value,
                    player.PlayerCombatState?.TurnNumber ?? -1, out int turnNumber, out error))
            {
                return CoopNativeDispatchResult.Rejected(error);
            }
            action = new EndPlayerTurnAction(player, turnNumber);
        }
        else
        {
            if (!NativeCardEncoding.TryParse(request.Value,
                    out NativeCardEncoding cardEncoding, out error))
            {
                return CoopNativeDispatchResult.Rejected(error);
            }
            if (player.PlayerCombatState is not { Phase: PlayerTurnPhase.Play } combat)
                return CoopNativeDispatchResult.Rejected("native_combat_play_phase_unavailable");

            CardModel? card = combat.Hand.Cards.FirstOrDefault(candidate =>
            {
                try
                {
                    return NetCombatCardDb.Instance.TryGetCardId(candidate, out uint cardId)
                        && cardId == cardEncoding.CardId;
                }
                catch
                {
                    return false;
                }
            });
            if (card is null)
                return CoopNativeDispatchResult.Rejected("native_card_identity_not_in_hand");

            Creature? target = null;
            if (cardEncoding.TargetId.HasValue)
            {
                target = CombatManager.Instance.DebugOnlyGetState()?.Enemies
                    .FirstOrDefault(enemy => enemy.CombatId == cardEncoding.TargetId.Value);
                if (target is null)
                    return CoopNativeDispatchResult.Rejected("native_card_target_not_found");
            }
            if (!card.CanPlay() || !card.CanPlayTargeting(target))
                return CoopNativeDispatchResult.Rejected("native_card_not_legal");

            NetCombatCard netCard;
            try
            {
                netCard = NetCombatCard.FromModel(card);
            }
            catch
            {
                return CoopNativeDispatchResult.Rejected("native_card_wire_identity_unavailable");
            }
            action = new PlayCardAction(player, netCard, card.Id, cardEncoding.TargetId);
        }
        if (!CanRetainPending(request.OperationId)) return CoopNativeDispatchResult.Rejected("native_pending_capacity_exhausted");
        _pending[request.OperationId] = NativePendingOperation.ForAction(
            request.OperationId, request.ActionKind, _hostSequence, _nativeChecksumOrdinal, action,
            player, _authorityEpoch, participantNativeIds);
        try
        {
            // RequestEnqueue is the first-party producer. It serializes the concrete native
            // action (end-turn or card play) and the remote ActionQueueSynchronizer consumes it.
            synchronizer.RequestEnqueue(action);
            return CoopNativeDispatchResult.Accepted();
        }
        catch
        {
            // The request may have reached the native queue before an exception surfaced. Keep
            // it pending so recovery inspects this operation instead of retrying it.
            return CoopNativeDispatchResult.Unknown("native_action_dispatch_outcome_unknown");
        }
    }

}
