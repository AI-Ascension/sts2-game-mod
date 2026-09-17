// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Bounded read adapter for owners that already have a host-thread observation callback.
/// The adapter owns no CardModel references and returns an unavailable result when the host
/// thread cannot complete the read synchronously.
/// </summary>
internal sealed class LiveCardSourceReadAdapter
{
    private readonly LiveCombatSource _source;
    private readonly IRuntimeV3HostThread _thread;
    private LiveCardCapturedSnapshot? _retained;

    internal LiveCardSourceReadAdapter(LiveCombatSource source, IRuntimeV3HostThread thread)
    {
        _source = source ?? throw new ArgumentNullException(nameof(source));
        _thread = thread ?? throw new ArgumentNullException(nameof(thread));
    }

    /// <summary>
    /// Invokes the source capture through the existing host-thread owner. A caller must provide
    /// the authenticated instance and content-manifest identities; this adapter invents neither.
    /// </summary>
    internal LiveCardCapturedSnapshot Read(string instanceId, string contentManifest)
    {
        if (string.IsNullOrEmpty(instanceId) || string.IsNullOrEmpty(contentManifest))
            return LiveCardCapturedSnapshot.Unavailable("identity_unavailable");

        LiveCardCapturedSnapshot? result = null;
        try
        {
            _thread.Enqueue(() => result = _source.CaptureLiveCardSnapshot(
                instanceId, contentManifest));
        }
        catch (Exception)
        {
            return LiveCardCapturedSnapshot.Unavailable("host_thread_unavailable");
        }

        _retained = result is { Available: true } available ? available : null;
        return result ?? LiveCardCapturedSnapshot.Unavailable("host_read_outcome_unknown");
    }

    /// <summary>
    /// Returns the immutable observation retained by the preceding authenticated read. A
    /// selector carrying an instance epoch must never trigger a fresh capture, because a fresh
    /// capture advances the source epoch and would invalidate the selector it is checking.
    /// </summary>
    internal LiveCardCapturedSnapshot ReadRetained(
        string instanceId,
        string contentManifest,
        ulong epoch)
    {
        LiveCardCapturedSnapshot? retained = _retained;
        return retained is { Available: true }
            && retained.InstanceId == instanceId
            && retained.ContentManifest == contentManifest
            && retained.Epoch == epoch
            ? retained
            : LiveCardCapturedSnapshot.Unavailable("stale_snapshot");
    }

    /// <summary>Invalidates source-owned CardModel occurrence handles on a lifecycle boundary.</summary>
    internal void Invalidate()
    {
        try
        {
            _thread.Enqueue(_source.InvalidateLiveCardSnapshot);
            _retained = null;
        }
        catch (Exception)
        {
            // An unavailable host thread is already fail-closed; no stale snapshot is exposed.
        }
    }
}
