// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Models;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.TreasureRelicPicking;
using MegaCrit.Sts2.Core.Events;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Helpers;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// First-party semantic effect witnesses. These hooks are installed on every native peer. The
/// host keeps the pending operation and exports a receipt only after the matching checksum from
/// every admitted peer; clients use the same callback and checksum schedule without a pending
/// gateway operation.
/// </summary>
internal sealed partial class InstalledNativeCoopHostPort
{
    private ActionExecutor? _effectActionExecutor;
    private EventSynchronizer? _effectEventSynchronizer;
    private MapSelectionSynchronizer? _effectMapSynchronizer;
    private TreasureRoomRelicSynchronizer? _effectRelicSynchronizer;
    private PlayerChoiceSynchronizer? _effectChoiceSynchronizer;
    private readonly HashSet<Player> _effectPlayers = new();
    private readonly HashSet<EventModel> _effectEventModels = new();
    private readonly HashSet<GameAction> _nativeChoiceActions = new();
    private readonly HashSet<GameAction> _actionChecksummed = new();
    private readonly Dictionary<GameAction, NativeMapActionCandidate> _nativeMapActions = new();
    private NativeEventEffectBatch? _nativeEventBatch;
    private NativeRelicEffectBatch? _nativeRelicBatch;

    private sealed class NativeEventEffectBatch
    {
        internal NativeEventEffectBatch(EventSynchronizer synchronizer)
        {
            Synchronizer = synchronizer;
            Expected = synchronizer.Events.ToHashSet();
        }

        internal EventSynchronizer Synchronizer { get; }
        internal HashSet<EventModel> Expected { get; }
        internal HashSet<EventModel> Observed { get; } = new();
        internal bool VoteObserved { get; set; }
        internal bool CheckpointScheduled { get; set; }

        internal bool IsComplete => Expected.Count > 0 && Expected.SetEquals(Observed);
    }

    private sealed class NativeRelicEffectBatch
    {
        internal NativeRelicEffectBatch(
            TreasureRoomRelicSynchronizer synchronizer,
            IEnumerable<RelicPickingResult> results)
        {
            Synchronizer = synchronizer;
            foreach (RelicPickingResult result in results)
            {
                if (result.type != RelicPickingResultType.Skipped && result.player is not null)
                    ExpectedObtains.Add((result.player.NetId, result.relic.Id));
            }
        }

        internal TreasureRoomRelicSynchronizer Synchronizer { get; }
        internal List<(ulong PlayerId, ModelId RelicId)> ExpectedObtains { get; } = new();
        internal List<(ulong PlayerId, ModelId RelicId)> Obtained { get; } = new();
        internal bool CheckpointScheduled { get; set; }

        internal bool IsComplete => ExpectedObtains.Count == 0;
    }

    private sealed class NativeMapActionCandidate
    {
        internal NativeMapActionCandidate(GameAction action, MapLocation source,
            int mapGenerationCount, MapCoord destination, MapSelectionSynchronizer synchronizer)
        {
            Action = action;
            Source = source;
            MapGenerationCount = mapGenerationCount;
            Destination = destination;
            Synchronizer = synchronizer;
        }

        internal GameAction Action { get; }
        internal MapLocation Source { get; }
        internal int MapGenerationCount { get; }
        internal MapCoord Destination { get; }
        internal MapSelectionSynchronizer Synchronizer { get; }
        internal bool CheckpointGenerated { get; set; }
    }

    /// <summary>
    /// Installs additive hooks on the installed synchronizers and action executor. Rebinding is
    /// identity based so a disposed run cannot leave callbacks attached to a later run.
    /// </summary>
    private void EnsureEffectHooks(RunManager manager)
    {
        ActionExecutor? actionExecutor = manager.ActionExecutor;
        EventSynchronizer? eventSynchronizer = manager.EventSynchronizer;
        MapSelectionSynchronizer? mapSynchronizer = manager.MapSelectionSynchronizer;
        TreasureRoomRelicSynchronizer? relicSynchronizer = manager.TreasureRoomRelicSynchronizer;
        PlayerChoiceSynchronizer? choiceSynchronizer = manager.PlayerChoiceSynchronizer;
        RunState? runState = manager.IsInProgress ? manager.DebugOnlyGetState() : null;

        if (ReferenceEquals(_effectActionExecutor, actionExecutor)
            && ReferenceEquals(_effectEventSynchronizer, eventSynchronizer)
            && ReferenceEquals(_effectMapSynchronizer, mapSynchronizer)
            && ReferenceEquals(_effectRelicSynchronizer, relicSynchronizer)
            && ReferenceEquals(_effectChoiceSynchronizer, choiceSynchronizer)
            && runState is not null
            && _effectPlayers.Count == runState.Players.Count
            && runState.Players.All(_effectPlayers.Contains))
        {
            RefreshEventModelHooks(eventSynchronizer);
            return;
        }

        if (_effectActionExecutor is not null)
        {
            _effectActionExecutor.BeforeActionExecuted -= OnNativeActionStarting;
            _effectActionExecutor.AfterActionExecuted -= OnNativeActionExecuted;
        }
        if (_effectEventSynchronizer is not null)
        {
            _effectEventSynchronizer.PlayerVoteChanged -= OnNativeEventVoteChanged;
            foreach (EventModel eventModel in _effectEventModels)
                eventModel.StateChanged -= OnNativeEventStateChanged;
        }
        if (_effectMapSynchronizer is not null)
            _effectMapSynchronizer.PlayerVotesCleared -= OnNativeMapVotesCleared;
        if (_effectRelicSynchronizer is not null)
            _effectRelicSynchronizer.RelicsAwarded -= OnNativeRelicsAwarded;
        if (_effectChoiceSynchronizer is not null)
            _effectChoiceSynchronizer.PlayerChoiceReceived -= OnNativePlayerChoiceReceived;
        foreach (Player player in _effectPlayers)
            player.RelicObtained -= OnNativeRelicObtained;

        _effectActionExecutor = actionExecutor;
        _effectEventSynchronizer = eventSynchronizer;
        _effectMapSynchronizer = mapSynchronizer;
        _effectRelicSynchronizer = relicSynchronizer;
        _effectChoiceSynchronizer = choiceSynchronizer;
        _effectPlayers.Clear();
        _effectEventModels.Clear();
        _nativeChoiceActions.Clear();
        _actionChecksummed.Clear();
        _nativeMapActions.Clear();
        _nativeEventBatch = null;
        _nativeRelicBatch = null;

        if (_effectActionExecutor is not null)
        {
            _effectActionExecutor.BeforeActionExecuted += OnNativeActionStarting;
            _effectActionExecutor.AfterActionExecuted += OnNativeActionExecuted;
        }
        if (_effectEventSynchronizer is not null)
        {
            _effectEventSynchronizer.PlayerVoteChanged += OnNativeEventVoteChanged;
            RefreshEventModelHooks(_effectEventSynchronizer);
        }
        if (_effectMapSynchronizer is not null)
            _effectMapSynchronizer.PlayerVotesCleared += OnNativeMapVotesCleared;
        if (_effectRelicSynchronizer is not null)
            _effectRelicSynchronizer.RelicsAwarded += OnNativeRelicsAwarded;
        if (_effectChoiceSynchronizer is not null)
            _effectChoiceSynchronizer.PlayerChoiceReceived += OnNativePlayerChoiceReceived;
        if (runState is not null)
        {
            foreach (Player player in runState.Players)
            {
                player.RelicObtained += OnNativeRelicObtained;
                _effectPlayers.Add(player);
            }
        }
    }

    private void ResetEffectWitnesses()
    {
        _nativeChoiceActions.Clear();
        _actionChecksummed.Clear();
        _nativeMapActions.Clear();
        _nativeEventBatch = null;
        _nativeRelicBatch = null;
    }


}
