// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

internal enum SeededRunStandardHostStatus
{
    Accepted,
    Settled,
    Rejected,
    Unknown
}

internal sealed record SeededRunStandardObservation(
    bool RunStarted,
    bool HostReady,
    ulong Generation,
    string CanonicalSeed,
    string SelectedContextDigest,
    string PhaseBefore,
    string PhaseAfter,
    string CompatibilityIdentity);

internal sealed record SeededRunStandardEffectWitness(
    string Kind,
    ulong Generation,
    string CanonicalSeed);

/// <summary>Receipt retained by the host for one seeded-run operation.</summary>
internal sealed record SeededRunStandardHostReceipt(
    string OperationId,
    string RequestedSeed,
    string RunMode,
    SeededRunSelectionContext? SelectedContext,
    SeededRunStandardHostStatus Status,
    string? CanonicalSeed,
    SeededRunStandardObservation? Observation,
    SeededRunStandardEffectWitness? EffectWitness,
    string? ErrorCode,
    ulong RequestGeneration);
