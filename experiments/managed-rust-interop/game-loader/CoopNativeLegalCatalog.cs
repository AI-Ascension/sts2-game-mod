// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// A producer-owned catalog paired with one host observation. Each entry is an exact value
/// accepted by the native dispatch encoder; no operation or model decision crosses this seam.
/// </summary>
internal sealed record CoopNativeLegalCatalog(
    ulong HostGeneration,
    string ActorPeerId,
    IReadOnlyList<CoopNativeLegalAction> Actions,
    IReadOnlyList<CoopNativeLegalVote> Votes)
{
    internal static CoopNativeLegalCatalog Empty(CoopHostObservation observation) =>
        new(observation.HostGeneration, observation.LocalPeerId,
            Array.Empty<CoopNativeLegalAction>(), Array.Empty<CoopNativeLegalVote>());

    internal bool Validate(out string error)
    {
        if (HostGeneration > RuntimeV3GameplayContract.MaxGeneration
            || !CoopPeerIdentity.IsOpaque(ActorPeerId)
            || Actions.Count > RuntimeV3GameplayContract.MaxLegalActions
            || Votes.Count > RuntimeV3GameplayContract.MaxLegalActions)
        {
            error = "native co-op legal catalog is outside its bounds";
            return false;
        }

        var ids = new HashSet<string>(StringComparer.Ordinal);
        foreach (CoopNativeLegalAction action in Actions)
        {
            if (!action.Validate(out error) || !ids.Add(action.ActionId))
            {
                error = "native co-op legal action catalog is invalid or duplicated";
                return false;
            }
        }

        foreach (CoopNativeLegalVote vote in Votes)
        {
            if (!vote.Validate(ActorPeerId, out error)
                || !ids.Add(vote.ProviderActionId))
            {
                error = "native co-op legal vote catalog is invalid or duplicated";
                return false;
            }
        }

        error = string.Empty;
        return true;
    }
}

internal sealed record CoopNativeLegalAction(
    string ActionId,
    string Kind,
    string? TargetPeer)
{
    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(ActionId)
            || Kind is not ("play_card" or "end_turn" or "select_card"
                or "choose_reward" or "confirm_selection")
            || TargetPeer is not null && !CoopPeerIdentity.IsOpaque(TargetPeer))
        {
            error = "native co-op legal action is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }
}

internal sealed record CoopNativeLegalVote(
    string ProposalId,
    string VoterPeer,
    string Choice)
{
    internal string ProviderActionId => $"vote:{ProposalId}:{Choice}";

    internal bool Validate(string actorPeerId, out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(ProposalId)
            || !CoopPeerIdentity.IsOpaque(VoterPeer)
            || !string.Equals(VoterPeer, actorPeerId, StringComparison.Ordinal)
            || !RuntimeV3GameplayContract.IsIdentity(Choice)
            || !RuntimeV3GameplayContract.IsIdentity(ProviderActionId))
        {
            error = "native co-op legal vote is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }
}
