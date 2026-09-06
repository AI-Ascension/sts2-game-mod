// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplayRecoveryStore
{
    private static readonly IReadOnlyList<string> RecoveryStateValues =
        Array.AsReadOnly(new[] { "durable_receipt" });

    private sealed class OperationPayload
    {
        public string InstanceId { get; set; } = string.Empty;
        public string SessionId { get; set; } = string.Empty;
        public string LeaseId { get; set; } = string.Empty;
        public ulong LeaseEpoch { get; set; }
        public string OperationId { get; set; } = string.Empty;
        public string PayloadDigest { get; set; } = string.Empty;
        public string CanonicalAction { get; set; } = string.Empty;
        public string StateId { get; set; } = string.Empty;
        public string RecoveryState { get; set; } = string.Empty;
        public string Status { get; set; } = string.Empty;
        public bool WasDispatched { get; set; }
        public string ActionId { get; set; } = string.Empty;
        public string ActionKind { get; set; } = string.Empty;
        public string? ActionValue { get; set; }
        public string? ActionTargetId { get; set; }
        public ulong ActionGeneration { get; set; }
        public string? TicketId { get; set; }
        public string? TicketPayloadDigest { get; set; }
        public string? TicketBootId { get; set; }
        public string? TicketInstanceIncarnation { get; set; }
        public ulong TicketLeaseEpoch { get; set; }
        public string? TicketHostFenceId { get; set; }
        public ulong TicketAuthorityGeneration { get; set; }
        public ulong TicketFenceGeneration { get; set; }
        public string? TicketExpiresAt { get; set; }
        public string? TicketState { get; set; }
        public string? WitnessOperationId { get; set; }
        public string? WitnessActionId { get; set; }
        public string? WitnessActionKind { get; set; }
        public ulong WitnessFromGeneration { get; set; }
        public ulong WitnessToGeneration { get; set; }
        public string? WitnessStateId { get; set; }
        public string? WitnessEffectKind { get; set; }
        public string? ErrorCode { get; set; }
        public string CreatedAt { get; set; } = string.Empty;
        public string UpdatedAt { get; set; } = string.Empty;

        public OperationPayload() { }

        internal OperationPayload(RuntimeV3HostOperationRecord record)
        {
            InstanceId = record.Operation.InstanceId;
            SessionId = record.Operation.SessionId;
            LeaseId = record.Operation.LeaseId;
            LeaseEpoch = record.Operation.LeaseEpoch;
            OperationId = record.Operation.OperationId;
            PayloadDigest = record.PayloadDigest;
            CanonicalAction = record.CanonicalAction;
            StateId = record.StateId;
            RecoveryState = record.RecoveryState.ToString();
            Status = record.Status.ToString();
            WasDispatched = record.WasDispatched;
            ActionId = record.Action.ActionId;
            ActionKind = record.Action.Kind;
            ActionValue = record.Action.Value;
            ActionTargetId = record.Action.TargetId;
            ActionGeneration = record.Action.Generation;
            ErrorCode = record.ErrorCode;
            CreatedAt = record.CreatedAt.ToUniversalTime().ToString("O");
            UpdatedAt = record.UpdatedAt.ToUniversalTime().ToString("O");
            if (record.Ticket is { } ticket)
            {
                TicketId = ticket.TicketId;
                TicketPayloadDigest = ticket.PayloadDigest;
                TicketBootId = ticket.BootId;
                TicketInstanceIncarnation = ticket.InstanceIncarnation;
                TicketLeaseEpoch = ticket.LeaseEpoch;
                TicketHostFenceId = ticket.HostFenceId;
                TicketAuthorityGeneration = ticket.AuthorityGeneration;
                TicketFenceGeneration = ticket.FenceGeneration;
                TicketExpiresAt = ticket.ExpiresAt.ToUniversalTime().ToString("O");
                TicketState = ticket.State.ToString();
            }
            if (record.Witness is { } witness)
            {
                WitnessOperationId = witness.Operation.OperationId;
                WitnessActionId = witness.Action.ActionId;
                WitnessActionKind = witness.Action.Kind;
                WitnessFromGeneration = witness.FromGeneration;
                WitnessToGeneration = witness.ToGeneration;
                WitnessStateId = witness.StateId;
                WitnessEffectKind = witness.EffectKind;
            }
        }

        internal bool TryRestore(out RuntimeV3HostOperationRecord? record)
        {
            record = null;
            if (!RuntimeV3GameplayRecoveryContract.IsUuid(InstanceId)
                || !RuntimeV3GameplayRecoveryContract.IsUuid(SessionId)
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(LeaseId)
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(OperationId)
                || LeaseEpoch == 0 || LeaseEpoch > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || !RuntimeV3GameplayRecoveryContract.IsDigest(PayloadDigest)
                || CanonicalAction.Length == 0
                || CanonicalAction.Length > RuntimeV3GameplayRecoveryContract.MaxActionBytes
                || !RuntimeV3GameplayContract.IsIdentity(StateId)
                || !RuntimeV3GameplayContract.IsIdentity(ActionId)
                || !RuntimeV3GameplayContract.IsIdentity(ActionKind)
                || ActionGeneration > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || (ErrorCode is not null && !RuntimeV3GameplayContract.IsIdentity(ErrorCode)))
            {
                return false;
            }

            if (!RuntimeV3GameplayRecoveryCanonical.TryRead(
                    CanonicalAction, PayloadDigest, ActionGeneration,
                    out LegalActionReference? parsedAction)
                || parsedAction is null)
            {
                return false;
            }
            LegalActionReference action = parsedAction;
            if (action.ActionId != ActionId || action.Kind != ActionKind
                || action.Value != ActionValue || action.TargetId != ActionTargetId
                || action.Generation != ActionGeneration
                || !Enum.TryParse(RecoveryState, out RuntimeV3HostOperationState recoveryState)
                || !Enum.TryParse(Status, out RuntimeV3DispatchStatus status)
                || !Enum.IsDefined(recoveryState)
                || !Enum.IsDefined(status)
                || !DateTimeOffset.TryParse(
                    CreatedAt, null, System.Globalization.DateTimeStyles.RoundtripKind,
                    out DateTimeOffset created)
                || !DateTimeOffset.TryParse(
                    UpdatedAt, null, System.Globalization.DateTimeStyles.RoundtripKind,
                    out DateTimeOffset updated)
                || !RuntimeV3GameplayRecoveryContract.IsTimestamp(created)
                || !RuntimeV3GameplayRecoveryContract.IsTimestamp(updated))
            {
                return false;
            }

            RuntimeV3OperationKey operation = new(InstanceId, SessionId, LeaseId, LeaseEpoch, OperationId);
            RuntimeV3GameplayObservation before = RecoveryObservation(StateId, ActionGeneration);
            if (!TryRestoreTicket(operation, out RuntimeV3HostAdmissionTicket? ticket)
                || !TryRestoreWitness(operation, action, out RuntimeV3TransitionWitness? witness))
            {
                return false;
            }
            RuntimeV3GameplayObservation? observation = witness is null
                ? null
                : RecoveryObservation(witness.StateId, witness.ToGeneration);
            record = new RuntimeV3HostOperationRecord(
                operation,
                PayloadDigest,
                CanonicalAction,
                StateId,
                action,
                before,
                Array.Empty<LegalActionReference>(),
                recoveryState,
                status,
                WasDispatched,
                ticket,
                observation,
                witness,
                ErrorCode,
                created,
                updated);
            return true;
        }

        private bool TryRestoreTicket(
            RuntimeV3OperationKey operation, out RuntimeV3HostAdmissionTicket? ticket)
        {
            ticket = null;
            bool absent = TicketId is null && TicketPayloadDigest is null && TicketBootId is null
                && TicketInstanceIncarnation is null && TicketHostFenceId is null
                && TicketExpiresAt is null && TicketState is null;
            if (absent)
            {
                return true;
            }
            if (TicketId is null || TicketPayloadDigest is null || TicketBootId is null
                || TicketInstanceIncarnation is null || TicketHostFenceId is null
                || TicketExpiresAt is null || TicketState is null
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(TicketId)
                || !RuntimeV3GameplayRecoveryContract.IsDigest(TicketPayloadDigest)
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(TicketBootId)
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(TicketInstanceIncarnation)
                || !RuntimeV3GameplayRecoveryContract.IsRandomUuid(TicketHostFenceId)
                || TicketLeaseEpoch == 0 || TicketLeaseEpoch > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || TicketAuthorityGeneration == 0
                || TicketAuthorityGeneration > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || TicketFenceGeneration == 0
                || TicketFenceGeneration > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || !Enum.TryParse(TicketState, out RuntimeV3HostTicketState state)
                || !Enum.IsDefined(state)
                || !DateTimeOffset.TryParse(
                    TicketExpiresAt, null, System.Globalization.DateTimeStyles.RoundtripKind,
                    out DateTimeOffset expires)
                || !RuntimeV3GameplayRecoveryContract.IsTimestamp(expires)
                || TicketLeaseEpoch != operation.LeaseEpoch
                || TicketPayloadDigest != PayloadDigest)
            {
                return false;
            }
            ticket = new RuntimeV3HostAdmissionTicket(
                TicketId, operation, TicketPayloadDigest, TicketBootId, TicketInstanceIncarnation,
                TicketLeaseEpoch, TicketHostFenceId, TicketAuthorityGeneration,
                TicketFenceGeneration, expires, state);
            return true;
        }

        private bool TryRestoreWitness(
            RuntimeV3OperationKey operation,
            LegalActionReference action,
            out RuntimeV3TransitionWitness? witness)
        {
            witness = null;
            bool absent = WitnessOperationId is null && WitnessActionId is null
                && WitnessActionKind is null && WitnessStateId is null && WitnessEffectKind is null;
            if (absent)
            {
                return true;
            }
            if (WitnessOperationId is null || WitnessActionId is null || WitnessActionKind is null
                || WitnessStateId is null || WitnessEffectKind is null
                || WitnessOperationId != operation.OperationId
                || WitnessActionId != action.ActionId || WitnessActionKind != action.Kind
                || !RuntimeV3GameplayContract.IsIdentity(WitnessStateId)
                || !RuntimeV3GameplayContract.IsIdentity(WitnessEffectKind)
                || WitnessFromGeneration > RuntimeV3GameplayRecoveryContract.MaxWireInteger
                || WitnessToGeneration > RuntimeV3GameplayRecoveryContract.MaxWireInteger)
            {
                return false;
            }
            witness = new RuntimeV3TransitionWitness(
                operation, action, WitnessFromGeneration, WitnessToGeneration,
                WitnessStateId, WitnessEffectKind);
            return true;
        }
    }

    private static RuntimeV3GameplayObservation RecoveryObservation(string stateId, ulong generation) =>
        new(
            stateId,
            generation,
            null,
            new RuntimeV3GameplayPlayer(
                0,
                0,
                0,
                0,
                Array.Empty<RuntimeV3GameplayCard>(),
                Array.Empty<RuntimeV3GameplayCard>(),
                Array.Empty<RuntimeV3GameplayCard>(),
                Array.Empty<RuntimeV3GameplayCard>()),
            RuntimeV3GameplayState.Recovery,
            RecoveryStateValues,
            Array.Empty<RuntimeV3GameplayEnemy>())
        {
            IsActionable = false,
            InputEnabled = false,
            ModalBlocking = true
        };
}
