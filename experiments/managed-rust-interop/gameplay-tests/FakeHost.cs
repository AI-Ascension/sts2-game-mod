// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

// Synthetic independent host completion oracle. No game, profile, process or network access.
internal sealed class FakeHost : IRuntimeV3HostSource
{
    internal ulong Generation { get; set; } = 1;
    internal int Dispatches { get; private set; }
    internal bool Complete { get; set; }
    internal bool ThrowReads { get; set; }
    internal bool WrongOperation { get; set; }
    internal bool WrongAction { get; set; }
    private RuntimeV3OperationKey? _operation;
    private LegalActionReference? _action;
    private RuntimeV3GameplayObservation? _completedObservation;

    public RuntimeV3GameplayObservation Observe() => ThrowReads
        ? throw new InvalidOperationException("synthetic unavailable host")
        : RuntimeV3GameplayFixtures.CombatObservation(Generation);

    public IReadOnlyList<LegalActionReference> LegalActions(RuntimeV3GameplayObservation observation) =>
        ThrowReads ? throw new InvalidOperationException("synthetic unavailable catalog")
            : new[] { RuntimeV3GameplayFixtures.EndTurn(observation.Generation) };

    public bool Dispatch(RuntimeV3OperationKey operation, LegalActionReference action)
    {
        Dispatches++;
        _operation = operation;
        _action = action;
        Generation++;
        _completedObservation = RuntimeV3GameplayFixtures.CombatObservation(Generation);
        return true;
    }

    public RuntimeV3HostCompletion? Completion(RuntimeV3OperationKey operation, LegalActionReference action)
    {
        if (!Complete || operation != _operation || action != _action || _completedObservation is null)
        {
            return null;
        }
        return new RuntimeV3HostCompletion(_completedObservation,
            new RuntimeV3TransitionWitness(
                WrongOperation ? operation with { OperationId = "other-operation" } : operation,
                WrongAction ? action with { Kind = "rest" } : action,
                action.Generation, _completedObservation.Generation, _completedObservation.StateId, "turn_ended"),
            new[] { RuntimeV3GameplayFixtures.EndTurn(_completedObservation.Generation) });
    }
}

internal sealed class TestQueue : IRuntimeV3HostThread
{
    private readonly Queue<Action> _pending = new();
    internal bool Deferred { get; init; }
    internal int PendingCount => _pending.Count;
    public void Enqueue(Action work)
    {
        if (Deferred) { _pending.Enqueue(work); }
        else { work(); }
    }
    internal void Run()
    {
        while (_pending.TryDequeue(out Action? work)) { work(); }
    }
}
