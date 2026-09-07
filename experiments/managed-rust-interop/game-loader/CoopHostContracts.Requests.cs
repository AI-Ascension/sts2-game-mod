// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record CoopLocalActionRequest(
    string OperationId,
    ulong ExpectedHostGeneration,
    string ActorPeerId,
    string ActionKind,
    string? Value,
    string? TargetPeerId)
{
    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(OperationId)
            || ExpectedHostGeneration > RuntimeV3GameplayContract.MaxGeneration
            || !CoopPeerIdentity.IsOpaque(ActorPeerId)
            || !RuntimeV3GameplayContract.IsIdentity(ActionKind)
            || Value is not null && !RuntimeV3GameplayContract.IsIdentity(Value)
            || TargetPeerId is not null && !CoopPeerIdentity.IsOpaque(TargetPeerId))
        {
            error = "native co-op local action is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }
}

internal sealed record CoopSharedVoteRequest(
    string OperationId,
    string ProposalId,
    ulong ExpectedHostGeneration,
    string VoterPeerId,
    CoopVoteDomain Domain,
    string Choice)
{
    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(OperationId)
            || !RuntimeV3GameplayContract.IsIdentity(ProposalId)
            || ExpectedHostGeneration > RuntimeV3GameplayContract.MaxGeneration
            || !CoopPeerIdentity.IsOpaque(VoterPeerId)
            || !Enum.IsDefined(Domain)
            || !RuntimeV3GameplayContract.IsIdentity(Choice))
        {
            error = "native co-op shared vote is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }
}

internal sealed record CoopEffectWitness(
    string OperationId,
    string EffectId,
    string EffectKind,
    ulong FromHostGeneration,
    ulong ToHostGeneration,
    string StateDigest)
{
    // Canonical native authority identity; unlike AuthorityEpoch this may be compared between
    // host and client observations when both saw the same lobby and native host.
    internal string AuthorityId { get; init; } = "authority:test";
    // Process-local adapter lifecycle fence. It is intentionally not cross-process stable.
    internal string AuthorityEpoch { get; init; } = "epoch:test";
    internal string CheckpointId { get; init; } = "checkpoint:test";

    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(OperationId)
            || !RuntimeV3GameplayContract.IsIdentity(EffectId)
            || !RuntimeV3GameplayContract.IsIdentity(EffectKind)
            || FromHostGeneration >= ToHostGeneration
            || ToHostGeneration > RuntimeV3GameplayContract.MaxGeneration
            || !CoopNativeValueRules.IsDigest(StateDigest)
            || !RuntimeV3GameplayContract.IsIdentity(AuthorityId)
            || !RuntimeV3GameplayContract.IsIdentity(AuthorityEpoch)
            || !RuntimeV3GameplayContract.IsIdentity(CheckpointId))
        {
            error = "native co-op effect witness is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }

}

internal sealed record CoopNativeDispatchResult(
    CoopOutcome Outcome,
    CoopEffectWitness? Effect,
    string? ErrorCode)
{
    internal static CoopNativeDispatchResult Accepted() =>
        new(CoopOutcome.Accepted, null, null);

    internal static CoopNativeDispatchResult Rejected(string errorCode) =>
        new(CoopOutcome.Rejected, null, errorCode);

    internal static CoopNativeDispatchResult Unknown(string errorCode) =>
        new(CoopOutcome.Unknown, null, errorCode);
}

/// <summary>
/// Host-specific adapter implemented by the managed first-party producer. All methods are
/// called from the Godot game thread; the runtime below owns fencing and idempotency.
/// </summary>
internal interface ICoopNativeHostPort
{
    CoopHostObservation Observe();
    bool TryResolvePeer(string opaquePeerId, out CoopNativePeerBinding binding);
    // A native synchronizer may accept an operation while the installed checksum remains
    // unreadable. The operation must then stay accepted/unknown until Reconcile proves the
    // native effect and all peer checkpoints converge. Test ports default to fail-closed.
    bool MayDispatchWithUnknownDigest(CoopHostObservation observation) => false;
    CoopNativeDispatchResult DispatchLocalAction(CoopLocalActionRequest request);
    CoopNativeDispatchResult SubmitSharedVote(CoopSharedVoteRequest request);
    CoopNativeDispatchResult Rejoin(string opaquePeerId, ulong rejoinEpoch);
    CoopEffectWitness? Reconcile(string operationId);
}

/// <summary>
/// Internal one-to-one binding between a session-scoped opaque identity and the authenticated
/// native peer. This type is deliberately outside every exported observation/response record.
/// </summary>
internal sealed record CoopNativePeerBinding(
    string OpaquePeerId,
    ulong NativePeerId,
    bool Authenticated)
{
    internal bool Validate(out string error)
    {
        if (!CoopPeerIdentity.IsOpaque(OpaquePeerId) || !Authenticated)
        {
            error = "native peer binding is not authenticated or opaque";
            return false;
        }
        error = string.Empty;
        return true;
    }
}
