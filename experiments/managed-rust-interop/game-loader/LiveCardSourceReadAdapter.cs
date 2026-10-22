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
    private bool _invalidationPending;

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
        _retained = null;
        if (string.IsNullOrEmpty(instanceId) || string.IsNullOrEmpty(contentManifest))
            return LiveCardCapturedSnapshot.Unavailable("identity_unavailable");

        LiveCardCapturedSnapshot? result = null;
        try
        {
            _thread.Enqueue(() =>
            {
                if (_invalidationPending)
                {
                    _source.InvalidateLiveCardSnapshot();
                    _invalidationPending = false;
                }
                result = _source.CaptureLiveCardSnapshot(instanceId, contentManifest);
            });
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
        if (retained is not { Available: true }
            || retained.InstanceId != instanceId
            || retained.ContentManifest != contentManifest
            || retained.Epoch != epoch)
            return LiveCardCapturedSnapshot.Unavailable("stale_snapshot");

        bool current = false;
        try
        {
            _thread.Enqueue(() => current = _source.IsLiveCardSnapshotCurrent(retained));
        }
        catch (Exception)
        {
            _retained = null;
            return LiveCardCapturedSnapshot.Unavailable("host_thread_unavailable");
        }
        if (current)
            return retained;
        _retained = null;
        return LiveCardCapturedSnapshot.Unavailable("stale_snapshot");
    }

    /// <summary>Invalidates source-owned CardModel occurrence handles on a lifecycle boundary.</summary>
    internal void Invalidate()
    {
        _retained = null;
        _invalidationPending = true;
        try
        {
            _thread.Enqueue(() =>
            {
                _source.InvalidateLiveCardSnapshot();
                _invalidationPending = false;
            });
        }
        catch (Exception)
        {
            // No stale snapshot is exposed. The next successful read first retires old handles.
        }
    }
}
