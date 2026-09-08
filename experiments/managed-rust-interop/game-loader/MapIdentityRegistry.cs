// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Assigns bounded opaque identities to host map-point references.
/// </summary>
/// <remarks>
/// The registry is deliberately lifetime-scoped. Once its bound is reached it keeps returning
/// an explicit failure for that run/map pair; it is reset only after <see cref="EnsureMap"/>
/// observes a new run or map object. This prevents an exhausted registry from silently renaming
/// references during the same map lifetime. A new host map is therefore the explicit recovery
/// boundary.
/// </remarks>
internal sealed class MapIdentityRegistry
{
    private readonly Dictionary<object, string> _tokens =
        new(ReferenceEqualityComparer.Instance);
    private object? _runIdentity;
    private object? _mapIdentity;
    private string? _mapInstanceId;
    private uint _nextToken;
    private bool _exhausted;

    internal string EnsureMap(object run, object map)
    {
        if (!ReferenceEquals(run, _runIdentity) || !ReferenceEquals(map, _mapIdentity))
        {
            _runIdentity = run;
            _mapIdentity = map;
            _mapInstanceId = $"map-instance:{Guid.NewGuid():N}";
            _tokens.Clear();
            _nextToken = 0;
            _exhausted = false;
        }
        return _mapInstanceId!;
    }

    internal bool TryGetNodeIds<T>(string mapInstanceId, int act, IReadOnlyList<T> points,
        out Dictionary<object, string> ids, out string reason) where T : class
    {
        ids = new Dictionary<object, string>(points.Count, ReferenceEqualityComparer.Instance);
        if (_exhausted)
        {
            reason = "map_identity_registry_bound_exceeded";
            return false;
        }
        foreach (T point in points)
        {
            if (!_tokens.TryGetValue(point, out string? token))
            {
                if (_tokens.Count >= RuntimeMapV1Contract.MaxMapIdentityRegistryEntries)
                {
                    // Keep the failure sticky for this run/map lifetime. The caller must wait
                    // for EnsureMap to observe a different host map before trying again.
                    _exhausted = true;
                    reason = "map_identity_registry_bound_exceeded";
                    return false;
                }
                token = $"p{++_nextToken}";
                _tokens.Add(point, token);
            }
            ids.Add(point, $"map-node:{mapInstanceId}:act:{act}:ref:{token}");
        }
        reason = string.Empty;
        return true;
    }

}
