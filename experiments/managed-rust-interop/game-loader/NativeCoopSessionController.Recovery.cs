// SPDX-License-Identifier: MIT

using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Multiplayer.Game.Lobby;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class NativeCoopSessionController
{
    /// <summary>
    /// The rejoin response contains the authoritative serialized run. LoadRunLobby still owns
    /// the first-party multiplayer synchronizers, so recovery supplies the smallest listener
    /// needed by that supported setup without pretending that a fresh character-select lobby was
    /// created.
    /// </summary>
    private sealed class RecoveryLobbyListener : ILoadRunLobbyListener
    {
        private readonly ControllerState _state;

        internal RecoveryLobbyListener(ControllerState state)
        {
            _state = state;
        }

        public void PlayerConnected(ulong playerId)
        {
        }

        public void RemotePlayerDisconnected(ulong playerId)
        {
        }

        public Task<bool> ShouldAllowRunToBegin() => Task.FromResult(true);

        public void BeginRun()
        {
        }

        public void PlayerReadyChanged(ulong playerId)
        {
        }

        public void LocalPlayerDisconnected(NetErrorInfo error)
        {
            if (_state.IsRejoin && !_state.Completed)
                RejoinStatus = "failed:native client disconnected during run restore";
        }
    }
}
