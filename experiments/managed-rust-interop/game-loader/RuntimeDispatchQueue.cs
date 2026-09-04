// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading.Tasks;

namespace AiAscension.Sts2GameMod.Runtime;

// One lock makes admission, execution claim, timeout removal, and response publication atomic.
internal sealed class RuntimeDispatchQueue<T>
{
    private readonly object _gate = new();
    private readonly LinkedList<Pending> _pending = new();
    private readonly int _capacity;

    internal RuntimeDispatchQueue(int capacity)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(capacity);
        _capacity = capacity;
    }

    internal sealed class Pending(T value, TimeSpan lifetime)
    {
        internal T Value { get; } = value;
        internal long Started { get; } = Stopwatch.GetTimestamp();
        internal TimeSpan Lifetime { get; } = lifetime;
        internal LinkedListNode<Pending>? Node { get; set; }
        internal TaskCompletionSource<(int Status, string Response)> Completion { get; } =
            new(TaskCreationOptions.RunContinuationsAsynchronously);
    }

    internal Pending? Enqueue(T value, TimeSpan? lifetime = null)
    {
        lock (_gate)
        {
            if (_pending.Count >= _capacity)
            {
                return null;
            }
            Pending work = new(value, lifetime ?? TimeSpan.FromSeconds(5));
            work.Node = _pending.AddLast(work);
            return work;
        }
    }

    internal (int Status, string Response) Wait(Pending work, TimeSpan timeout)
    {
        TimeSpan remaining = work.Lifetime - Stopwatch.GetElapsedTime(work.Started);
        TimeSpan wait = remaining < timeout ? remaining : timeout;
        if (work.Completion.Task.Wait(wait > TimeSpan.Zero ? wait : TimeSpan.Zero))
        {
            return work.Completion.Task.Result;
        }
        lock (_gate)
        {
            if (work.Completion.Task.IsCompleted)
            {
                return work.Completion.Task.Result;
            }
            bool notStarted = work.Node != null;
            if (work.Node != null)
            {
                _pending.Remove(work.Node);
                work.Node = null;
            }
            // A started host call cannot be canceled or reported as rejected. Runtime-v1 has
            // no unknown receipt variant: use an owner-local transport failure, not an envelope.
            var result = (504, notStarted
                ? "{\"error_code\":\"main_thread_timeout_before_dispatch\"}"
                : "{\"error_code\":\"main_thread_outcome_unknown\"}");
            work.Completion.TrySetResult(result);
            return result;
        }
    }

    internal bool ProcessOne(Func<T, (int Status, string Response)> execute)
    {
        Pending work;
        lock (_gate)
        {
            if (_pending.First == null)
            {
                return false;
            }
            work = _pending.First.Value;
            _pending.RemoveFirst();
            work.Node = null;
            if (Stopwatch.GetElapsedTime(work.Started) >= work.Lifetime)
            {
                work.Completion.TrySetResult((504,
                    "{\"error_code\":\"main_thread_timeout_before_dispatch\"}"));
                return true;
            }
        }
        var response = execute(work.Value);
        lock (_gate)
        {
            work.Completion.TrySetResult(response);
        }
        return true;
    }
}
