// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopHostRuntime
{
    internal bool TryResolveNativePeer(ulong nativePeerId, out CoopNativePeerBinding binding) =>
        _port.TryResolveNativePeer(nativePeerId, out binding);

    internal CoopOperationReceipt DispatchLocalAction(CoopLocalActionRequest request) =>
        DispatchLocalAction(request, null);

    internal CoopOperationReceipt DispatchAuthenticatedLocalAction(
        CoopLocalActionRequest request, ulong nativePeerId) => DispatchLocalAction(request, nativePeerId);

    internal CoopOperationReceipt SubmitSharedVote(CoopSharedVoteRequest request) =>
        SubmitSharedVote(request, null);

    internal CoopOperationReceipt SubmitAuthenticatedSharedVote(
        CoopSharedVoteRequest request, ulong nativePeerId) => SubmitSharedVote(request, nativePeerId);

    private CoopOperationReceipt DispatchLocalAction(CoopLocalActionRequest request, ulong? nativePeerId) =>
        Dispatch(request, request.ActorPeerId, nativePeerId, () => _port.DispatchLocalAction(request), "invalid_local_action", "native_dispatch_outcome_unknown");

    private CoopOperationReceipt SubmitSharedVote(CoopSharedVoteRequest request, ulong? nativePeerId) =>
        Dispatch(request, request.VoterPeerId, nativePeerId, () => _port.SubmitSharedVote(request), "invalid_shared_vote", "native_vote_outcome_unknown");

    private CoopOperationReceipt Dispatch(object request, string actorPeerId, ulong? nativePeerId,
        System.Func<CoopNativeDispatchResult> execute, string invalid, string unknown)
    {
        bool valid = request is CoopLocalActionRequest action ? action.Validate(out _) : ((CoopSharedVoteRequest)request).Validate(out _);
        string operationId = request is CoopLocalActionRequest local ? local.OperationId : ((CoopSharedVoteRequest)request).OperationId;
        ulong generation = request is CoopLocalActionRequest localRequest ? localRequest.ExpectedHostGeneration : ((CoopSharedVoteRequest)request).ExpectedHostGeneration;
        string fingerprint = request is CoopLocalActionRequest localFingerprint ? Fingerprint(localFingerprint) : Fingerprint((CoopSharedVoteRequest)request);
        if (!valid) return Rejected(operationId, 0, invalid);
        if (TryGetDuplicate(operationId, fingerprint, out CoopOperationReceipt duplicate)) return duplicate;
        CoopHostObservation before = Observe();
        if (!CanDispatch(before, generation, actorPeerId, nativePeerId, out string error)) return Rejected(operationId, before.HostGeneration, error);
        var accepted = new CoopOperationReceipt(operationId, fingerprint, CoopOutcome.Accepted, before.HostGeneration, null, null, before, null);
        if (_receipts.Count >= MaxReceipts || !_receipts.TryAdd(operationId, accepted)) return Rejected(operationId, before.HostGeneration, "receipt_capacity_exhausted");
        if (!TryPassExternalAdmission(out string admissionError)) { _receipts.TryRemove(operationId, out _); return Rejected(operationId, before.HostGeneration, admissionError); }
        CoopNativeDispatchResult result;
        try { result = execute(); } catch { result = CoopNativeDispatchResult.Unknown(unknown); }
        return FinalizeDispatch(operationId, before, result);
    }
}
