// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopHostRuntime
{
    private static string Fingerprint(CoopLocalActionRequest request) =>
        $"local|{request.ExpectedHostGeneration}|{request.ActorPeerId}|{request.ActionKind}|{request.Value}|{request.TargetPeerId}";

    private static string Fingerprint(CoopSharedVoteRequest request) =>
        $"vote|{request.ProposalId}|{request.ExpectedHostGeneration}|{request.VoterPeerId}|{request.Domain}|{request.Choice}";

    private static CoopOperationReceipt Rejected(string operationId,
        ulong generation, string errorCode)
    {
        // Rejected requests have no native snapshot. The caller must observe before retrying;
        // this placeholder is never exported as a successful host observation.
        var empty = new CoopHostObservation("unavailable", "peer:local", CoopHostRole.Unknown,
            "unknown", null, generation, "digest:unknown",
            new[] { new CoopPeerSnapshot("peer:local", true, false, generation, "digest:unknown") },
            true, null);
        return new CoopOperationReceipt(operationId, string.Empty, CoopOutcome.Rejected, generation,
            null, null, empty, errorCode);
    }
}
