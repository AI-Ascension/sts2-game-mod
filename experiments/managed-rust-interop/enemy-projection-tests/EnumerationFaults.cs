// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using MegaCrit.Sts2.Core.Entities.Creatures;

namespace AiAscension.Sts2GameMod.Runtime;

internal enum EnumerationFault { GetEnumerator, MoveBeforeFirst, MoveAfterFirst, Current, Dispose }

// Injects failures into each enumeration boundary; none of these is game implementation code.
internal sealed class FaultingEnemies(EnumerationFault fault) : IEnumerable<Creature>, IEnumerator<Creature>
{
    private int _position = -1;
    private bool _disposed;
    public Creature Current => fault == EnumerationFault.Current
        ? throw EnemyProjectionProbe.PrivateFailure() : EnemyProjectionProbe.Enemy(_position + 1);
    object IEnumerator.Current => Current;
    public IEnumerator<Creature> GetEnumerator() => fault == EnumerationFault.GetEnumerator
        ? throw EnemyProjectionProbe.PrivateFailure() : this;
    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    public bool MoveNext()
    {
        _position++;
        if (fault == EnumerationFault.MoveBeforeFirst
            || fault == EnumerationFault.MoveAfterFirst && _position == 1)
            throw EnemyProjectionProbe.PrivateFailure();
        return _position < 2;
    }
    public void Reset() => throw new NotSupportedException();
    public void Dispose()
    {
        GC.SuppressFinalize(this);
        if (_disposed) return;
        _disposed = true;
        if (fault == EnumerationFault.Dispose) throw EnemyProjectionProbe.PrivateFailure();
    }
}
