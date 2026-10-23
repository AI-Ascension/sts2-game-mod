// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Owned synchronous reads; deferred callbacks cannot publish or revive a snapshot.</summary>
internal sealed class LiveCardSourceReadAdapter
{
    private readonly LiveCombatSource _source;
    private readonly IRuntimeV3HostThread _thread;
    private readonly object _gate = new();
    private LiveCardCapturedSnapshot? _retained;
    private object _request = new();

    internal LiveCardSourceReadAdapter(LiveCombatSource source, IRuntimeV3HostThread thread)
    {
        _source = source ?? throw new ArgumentNullException(nameof(source));
        _thread = thread ?? throw new ArgumentNullException(nameof(thread));
    }

    internal LiveCardCapturedSnapshot Read(string instanceId, string contentManifest)
    {
        lock (_gate)
        {
            _retained = null;
            object request = _request = new object();
            bool open = true;
            LiveCardCapturedSnapshot? result = null;
            try
            {
                if (string.IsNullOrEmpty(instanceId) || string.IsNullOrEmpty(contentManifest))
                    return LiveCardCapturedSnapshot.Unavailable("identity_unavailable");
                _thread.Enqueue(() =>
                {
                    lock (_gate)
                    {
                        if (open && ReferenceEquals(request, _request))
                            result = _source.CaptureLiveCardSnapshot(instanceId, contentManifest);
                    }
                });
                _retained = result is { Available: true } ? result : null;
                return result ?? LiveCardCapturedSnapshot.Unavailable("host_read_outcome_unknown");
            }
            catch (Exception)
            {
                return LiveCardCapturedSnapshot.Unavailable("host_thread_unavailable");
            }
            finally
            {
                open = false;
                if (_retained is null) InvalidateSource(request);
            }
        }
    }

    /// <summary>Rechecks current host values without capturing or advancing the selected epoch.</summary>
    internal LiveCardCapturedSnapshot ReadRetained(
        string instanceId, string contentManifest, ulong epoch)
    {
        lock (_gate)
        {
            LiveCardCapturedSnapshot? retained = _retained;
            object request = _request;
            bool open = true;
            bool current = false;
            try
            {
                if (retained is { Available: true } && retained.InstanceId == instanceId
                    && retained.ContentManifest == contentManifest && retained.Epoch == epoch)
                    _thread.Enqueue(() =>
                    {
                        lock (_gate)
                        {
                            if (open && ReferenceEquals(request, _request))
                                current = _source.IsLiveCardSnapshotCurrent(retained);
                        }
                    });
            }
            catch (Exception) { current = false; }
            finally { open = false; }
            if (current) return retained!;
            _retained = null;
            _request = new object();
            InvalidateSource(_request);
            return LiveCardCapturedSnapshot.Unavailable("stale_snapshot");
        }
    }

    internal void Invalidate()
    {
        lock (_gate)
        {
            _retained = null;
            _request = new object();
            InvalidateSource(_request);
        }
    }

    private void InvalidateSource(object request)
    {
        try
        {
            _thread.Enqueue(() =>
            {
                lock (_gate)
                {
                    if (ReferenceEquals(request, _request)) _source.InvalidateLiveCardSnapshot();
                }
            });
        }
        catch (Exception)
        {
            // Host references can only be released by their owner. Retained DTO is already gone;
            // a later successful read replaces the fence, and deferred work is token-fenced.
        }
    }
}
