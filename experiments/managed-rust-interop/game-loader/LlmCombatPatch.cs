// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Narrow semantic combat patch. Policy remains outside the mod; this class only validates the
/// current host catalog and submits the already selected typed action to the host-thread owner.
/// </summary>
internal sealed class LlmCombatPatch
{
    private readonly RuntimeV3GameplayHost _host;

    internal LlmCombatPatch(RuntimeV3GameplayHost host)
    {
        _host = host;
    }

    internal bool TryDispatchCurrent(
        RuntimeV3OperationKey operation,
        RuntimeV3GameplayObservation observation,
        string actionId,
        out RuntimeV3DispatchReceipt? receipt)
    {
        foreach (LegalActionReference action in _host.LegalActions(observation))
        {
            if (string.Equals(action.ActionId, actionId, StringComparison.Ordinal))
            {
                receipt = _host.Dispatch(operation, observation, action);
                return true;
            }
        }

        receipt = null;
        return false;
    }
}
