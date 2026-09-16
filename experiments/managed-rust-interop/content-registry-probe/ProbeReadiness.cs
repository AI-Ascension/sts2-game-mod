// SPDX-License-Identifier: MIT

using MegaCrit.Sts2.Core.Modding;

namespace AiAscension.Sts2ModelDbRegistryProbe;

internal enum ProbeReadiness
{
    Wait,
    Ready,
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
        return ProbeReadiness.Ready;
    }

    internal static ProbeReadiness EvaluateRegistryCount(int registryCount) =>
        registryCount > 0 ? ProbeReadiness.Ready : ProbeReadiness.Wait;
}
