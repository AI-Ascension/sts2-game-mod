// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV3RecoveryRelease(
    string ReleaseDigest,
    string ConfigDigest,
    string ProfileDigest,
    string RuntimeV3SchemaDigest);

internal sealed record RuntimeV3HostFence(
    string DeploymentId,
    string InstanceId,
    string InstanceIncarnation,
    string BootId,
    ulong AuthorityGeneration,
    string LeaseId,
    ulong LeaseEpoch,
    string HostFenceId,
    ulong FenceGeneration,
    DateTimeOffset ExpiresAt);

internal sealed record RuntimeV3HostBootstrapRequest(
    RuntimeV3HostFence Fence,
    RuntimeV3RecoveryRelease Release,
    string Contract,
    string SchemaDigest,
    string Proof);

internal enum RuntimeV3HostTicketState
{
    Issued,
    Admitted,
    Executing,
    EffectWitnessRecorded,
    Settled,
    Rejected,
    Unknown
}

internal sealed record RuntimeV3HostAdmissionTicket(
    string TicketId,
    RuntimeV3OperationKey Operation,
    string PayloadDigest,
    string BootId,
    string InstanceIncarnation,
    ulong LeaseEpoch,
    string HostFenceId,
    ulong AuthorityGeneration,
    ulong FenceGeneration,
    DateTimeOffset ExpiresAt,
    RuntimeV3HostTicketState State);

internal enum RuntimeV3HostOperationState
{
    IntentRecorded,
    MayHaveBeenDispatched,
    Accepted,
    Settled,
    Rejected,
    Unknown,
    Reconciled
}

internal sealed record RuntimeV3HostOperationRecord(
    RuntimeV3OperationKey Operation,
    string PayloadDigest,
    string CanonicalAction,
    string StateId,
    LegalActionReference Action,
    RuntimeV3GameplayObservation Before,
    IReadOnlyList<LegalActionReference> LegalActions,
    RuntimeV3HostOperationState RecoveryState,
    RuntimeV3DispatchStatus Status,
    bool WasDispatched,
    RuntimeV3HostAdmissionTicket? Ticket,
    RuntimeV3GameplayObservation? Observation,
    RuntimeV3TransitionWitness? Witness,
    string? ErrorCode,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt);

internal sealed record RuntimeV3HistoricalOperation(
    RuntimeV3HostOperationRecord Record,
    bool MutationAuthorized);

internal enum RuntimeV3RecoveryAdmission
{
    New,
    Duplicate,
    Conflict,
    Blocked,
    Stale
}
