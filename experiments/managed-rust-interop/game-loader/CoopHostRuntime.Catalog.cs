// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopHostRuntime
{
    internal bool TryLegalCatalog(
        ulong expectedGeneration,
        string actorPeerId,
        out CoopNativeLegalCatalog? catalog,
        out CoopHostObservation? observation,
        out string errorCode)
    {
        catalog = null;
        observation = null;
        errorCode = "coop_native_legal_catalog_unavailable";
        CoopHostObservation before;
        try
        {
            before = Observe();
        }
        catch
        {
            return false;
        }

        if (before.HostGeneration != expectedGeneration
            || !string.Equals(before.LocalPeerId, actorPeerId,
                System.StringComparison.Ordinal))
        {
            errorCode = before.HostGeneration != expectedGeneration
                ? "coop_native_legal_catalog_stale_generation"
                : "coop_native_legal_catalog_actor_mismatch";
            return false;
        }

        try
        {
            if (!_port.TryResolvePeer(actorPeerId, out CoopNativePeerBinding binding)
                || !binding.Validate(out _)
                || !string.Equals(binding.OpaquePeerId, actorPeerId,
                    System.StringComparison.Ordinal))
            {
                errorCode = "coop_native_legal_catalog_actor_unknown";
                return false;
            }
        }
        catch
        {
            errorCode = "coop_native_legal_catalog_actor_unknown";
            return false;
        }

        try
        {
            CoopNativeLegalCatalog candidate = _port.LegalCatalog(before);
            if (candidate.HostGeneration != before.HostGeneration
                || !string.Equals(candidate.ActorPeerId, before.LocalPeerId,
                    System.StringComparison.Ordinal)
                || !candidate.Validate(out _))
            {
                return false;
            }

            catalog = candidate;
            observation = before;
            errorCode = string.Empty;
            return true;
        }
        catch
        {
            return false;
        }
    }
}
