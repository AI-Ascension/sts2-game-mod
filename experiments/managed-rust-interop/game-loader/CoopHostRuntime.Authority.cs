// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopHostRuntime
{
    private bool TryGetDuplicate(string operationId, string requestFingerprint,
        out CoopOperationReceipt duplicate)
    {
        duplicate = null!;
        if (!_receipts.TryGetValue(operationId, out CoopOperationReceipt? prior))
            return false;

        // A rejected placeholder may have no native observation to compare. It is still safe to
        // replay because it never reached the native port. Settled/accepted operations carry a
        // real authority fence; refresh it before replay so an operation ID cannot be reused
        // against a later lobby or host lineage.
        if (prior.Outcome is CoopOutcome.Settled or CoopOutcome.Accepted or CoopOutcome.Unknown)
        {
            try
            {
                CoopHostObservation current = Observe();
                if (!string.Equals(current.AuthorityId, prior.Observation.AuthorityId,
                        StringComparison.Ordinal)
                    || !string.Equals(current.AuthorityEpoch, prior.Observation.AuthorityEpoch,
                        StringComparison.Ordinal))
                {
                    CoopOperationReceipt stale = prior with
                    {
                        Outcome = CoopOutcome.Rejected,
                        AfterHostGeneration = null,
                        Effect = null,
                        Observation = current,
                        ErrorCode = "stale_native_authority"
                    };
                    _receipts[operationId] = stale;
                    duplicate = stale;
                    return true;
                }
            }
            catch
            {
                // Preserve idempotent replay when the native read is temporarily unavailable;
                // the caller still cannot cause a second mutation from this branch.
            }
        }

        duplicate = ValidateDuplicate(prior, requestFingerprint);
        return true;
    }

    private void PruneReceiptsForAuthority(CoopHostObservation current)
    {
        foreach (KeyValuePair<string, CoopOperationReceipt> entry in _receipts)
        {
            CoopOperationReceipt receipt = entry.Value;
            if (receipt.Outcome is not (CoopOutcome.Accepted or CoopOutcome.Unknown)
                || string.Equals(receipt.Observation.AuthorityId, current.AuthorityId,
                    StringComparison.Ordinal)
                && string.Equals(receipt.Observation.AuthorityEpoch, current.AuthorityEpoch,
                    StringComparison.Ordinal))
            {
                continue;
            }

            CoopOperationReceipt stale = receipt with
            {
                Outcome = CoopOutcome.Rejected,
                AfterHostGeneration = null,
                Effect = null,
                Observation = current,
                ErrorCode = "native_authority_changed"
            };
            _receipts.TryUpdate(entry.Key, stale, receipt);
        }
    }
}
