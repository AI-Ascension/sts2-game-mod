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

    internal RuntimeV3DispatchReceipt DispatchCurrent(
        string operationId,
        RuntimeV3GameplayObservation observation,
        string actionId)
    {
        foreach (LegalActionReference action in _host.LegalActions(observation))
        {
            if (string.Equals(action.ActionId, actionId, StringComparison.Ordinal))
            {
                return _host.Dispatch(operationId, observation, action);
            }
        }

        return new RuntimeV3DispatchReceipt(
            operationId,
            RuntimeV3DispatchStatus.Rejected,
            observation,
            null,
            "action_not_current",
            null);
    }
}
