// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplayHost
{
    private static bool TryRestoreReceipt(
        RuntimeV3HostOperationRecord record, out RuntimeV3DispatchReceipt? receipt)
    {
        receipt = null;
        RuntimeV3GameplayObservation before = record.Before;
        RuntimeV3GameplayObservation? observation = record.Observation;
        receipt = new RuntimeV3DispatchReceipt(
            record.Operation.OperationId,
            record.Status,
            observation,
            record.Witness,
            record.ErrorCode,
            record.Action,
            before,
            new List<LegalActionReference>(record.LegalActions).AsReadOnly(),
            record.WasDispatched);
        return true;
    }

    internal bool TryReplaceHostFence(
        RuntimeV3HostBootstrapRequest request, out string error)
    {
        if (_recovery is null)
        {
            error = "recovery_not_configured";
            return false;
        }
        return _recovery.TryReplaceFence(request, out error);
    }

    internal RuntimeV3HostFence? CurrentHostFence => _recovery?.CurrentFence;

    internal bool TryHistoricalOperation(
        RuntimeV3OperationKey operation,
        string payloadDigest,
        string proof,
        out RuntimeV3HistoricalOperation? historical,
        out string error)
    {
        historical = null;
        if (_recovery is null)
        {
            error = "recovery_not_configured";
            return false;
        }
        return _recovery.TryHistoricalLookup(operation, payloadDigest, proof, out historical, out error);
    }

    internal RuntimeV3GameplayRecoveryStore? RecoveryStore => _recovery;
}
