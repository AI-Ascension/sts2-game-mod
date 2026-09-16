// SPDX-License-Identifier: MIT

using MegaCrit.Sts2.Core.Modding;

namespace AiAscension.Sts2ModelDbRegistryProbe;

internal enum ProbeReadiness
{
    Wait,
    Observed,
    Refuse
}

internal static class ProbeReadinessGate
{
    internal static ProbeReadiness EvaluateManagerState(ModManagerState state)
    {
        if (state == ModManagerState.Skipped)
            return ProbeReadiness.Refuse;
        if (state != ModManagerState.Initialized)
            return ProbeReadiness.Wait;
        return ProbeReadiness.Observed;
    }

    internal static ProbeReadiness EvaluateRegistryCount(int registryCount) =>
        registryCount > 0 ? ProbeReadiness.Observed : ProbeReadiness.Wait;
}
