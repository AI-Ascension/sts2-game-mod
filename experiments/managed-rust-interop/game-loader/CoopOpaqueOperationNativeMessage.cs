// SPDX-License-Identifier: MIT

using System;
using System.Text;
using MegaCrit.Sts2.Core.Logging;
using MegaCrit.Sts2.Core.Multiplayer.Serialization;
using MegaCrit.Sts2.Core.Multiplayer.Transport;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Concrete first-party-message seam. The exact host discovers message subtypes dynamically;
/// this type intentionally has no hard-coded host type ID. Host dispatch, not an incoming message,
/// is the sole source of a settlement witness.
/// </summary>
public sealed class OpaqueOperationNativeMessage : INetMessage, IPacketSerializable
{
    internal const int MaximumOperationIdBytes = 96;
    // The frozen v1 envelope is bounded at 64 KiB.  The carrier keeps those bytes opaque so it
    // can preserve the original operation/session/lease identity without introducing a second
    // wire contract.
    internal const int MaximumPayloadBytes = 64 * 1024;
    internal const int MaximumWitnessBytes = 256;
    internal const int MaximumRejectionCodeBytes = 64;
    private const byte WireVersion = 1;

    public OpaqueOperationNativeMessage()
    {
        Kind = OpaqueOperationNativeMessageKind.Invalid;
        OriginalOperationId = string.Empty;
        Payload = Array.Empty<byte>();
        SettlementWitness = string.Empty;
        RejectionCode = string.Empty;
    }

    private OpaqueOperationNativeMessage(OpaqueOperationNativeMessageKind kind,
        string originalOperationId, byte[] payload, string settlementWitness, string rejectionCode)
    {
        Kind = kind;
        OriginalOperationId = originalOperationId;
        Payload = Copy(payload);
        SettlementWitness = settlementWitness;
        RejectionCode = rejectionCode;
    }

    public bool ShouldBroadcast => false;
    // The exact assembly's client-to-host request and checksum messages return mode value 2.
    public NetTransferMode Mode => (NetTransferMode)2;
    public LogLevel LogLevel => (LogLevel)0;
    public bool ShouldBuffer => true;

    internal OpaqueOperationNativeMessageKind Kind { get; private set; }
    internal string OriginalOperationId { get; private set; }
    internal byte[] Payload { get; private set; }
    internal string SettlementWitness { get; private set; }
    internal string RejectionCode { get; private set; }

    internal static OpaqueOperationNativeMessage Request(string originalOperationId, byte[] payload) =>
        new(OpaqueOperationNativeMessageKind.Request, originalOperationId, payload,
            string.Empty, string.Empty);

    internal static OpaqueOperationNativeMessage SettledReply(string originalOperationId,
        string settlementWitness) => new(OpaqueOperationNativeMessageKind.SettledReply,
            originalOperationId, Array.Empty<byte>(), settlementWitness, string.Empty);

    internal static OpaqueOperationNativeMessage PendingReply(string originalOperationId,
        string causalCode) => new(OpaqueOperationNativeMessageKind.PendingReply,
            originalOperationId, Array.Empty<byte>(), string.Empty, causalCode);

    internal static OpaqueOperationNativeMessage RejectedReply(string originalOperationId,
        string rejectionCode) => new(OpaqueOperationNativeMessageKind.RejectedReply,
            originalOperationId, Array.Empty<byte>(), string.Empty, rejectionCode);

    public void Serialize(PacketWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);
        if (!IsWellFormed())
            throw new InvalidOperationException("opaque operation message is outside its bounds");
        writer.WriteByte(WireVersion);
        writer.WriteByte((byte)Kind);
        WriteBoundedText(writer, OriginalOperationId);
        writer.WriteInt(Payload.Length);
        writer.WriteBytes(Payload, Payload.Length);
        WriteBoundedText(writer, SettlementWitness);
        WriteBoundedText(writer, RejectionCode);
    }

    public void Deserialize(PacketReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);
        ResetInvalid();
        if (reader.ReadByte() != WireVersion)
            return;
        OpaqueOperationNativeMessageKind kind = (OpaqueOperationNativeMessageKind)reader.ReadByte();
        if (!TryReadBoundedText(reader, MaximumOperationIdBytes, out string operationId)
            || !TryReadPayload(reader, out byte[] payload)
            || !TryReadBoundedText(reader, MaximumWitnessBytes, out string witness)
            || !TryReadBoundedText(reader, MaximumRejectionCodeBytes, out string rejectionCode))
            return;
        Kind = kind;
        OriginalOperationId = operationId;
        Payload = payload;
        SettlementWitness = witness;
        RejectionCode = rejectionCode;
        if (!IsWellFormed())
            ResetInvalid();
    }

    internal bool IsRequest() => Kind == OpaqueOperationNativeMessageKind.Request
        && IsOperationId(OriginalOperationId) && Payload.Length <= MaximumPayloadBytes
        && SettlementWitness.Length == 0 && RejectionCode.Length == 0;

    internal bool IsReply() => (Kind == OpaqueOperationNativeMessageKind.SettledReply
            && IsOperationId(OriginalOperationId) && Payload.Length == 0
            && IsOpaqueText(SettlementWitness, MaximumWitnessBytes) && RejectionCode.Length == 0)
        || (Kind == OpaqueOperationNativeMessageKind.PendingReply
            && IsOperationId(OriginalOperationId) && Payload.Length == 0
            && SettlementWitness.Length == 0
            && IsOpaqueText(RejectionCode, MaximumRejectionCodeBytes))
        || (Kind == OpaqueOperationNativeMessageKind.RejectedReply
            && IsOperationId(OriginalOperationId) && Payload.Length == 0
            && SettlementWitness.Length == 0
            && IsOpaqueText(RejectionCode, MaximumRejectionCodeBytes));

    private bool IsWellFormed() => IsRequest() || IsReply();

    private void ResetInvalid()
    {
        Kind = OpaqueOperationNativeMessageKind.Invalid;
        OriginalOperationId = string.Empty;
        Payload = Array.Empty<byte>();
        SettlementWitness = string.Empty;
        RejectionCode = string.Empty;
    }

    private static void WriteBoundedText(PacketWriter writer, string value)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(value);
        writer.WriteInt(bytes.Length);
        writer.WriteBytes(bytes, bytes.Length);
    }

    private static bool TryReadBoundedText(PacketReader reader, int maximumBytes, out string value)
    {
        value = string.Empty;
        int length = reader.ReadInt();
        if (length < 0 || length > maximumBytes)
            return false;
        byte[] bytes = new byte[length];
        reader.ReadBytes(bytes, length);
        value = Encoding.UTF8.GetString(bytes);
        return Encoding.UTF8.GetByteCount(value) == length;
    }

    private static bool TryReadPayload(PacketReader reader, out byte[] payload)
    {
        payload = Array.Empty<byte>();
        int length = reader.ReadInt();
        if (length < 0 || length > MaximumPayloadBytes)
            return false;
        payload = new byte[length];
        reader.ReadBytes(payload, length);
        return true;
    }

    private static byte[] Copy(byte[] value)
    {
        ArgumentNullException.ThrowIfNull(value);
        byte[] copy = new byte[value.Length];
        Array.Copy(value, copy, value.Length);
        return copy;
    }

    private static bool IsOperationId(string value) => IsOpaqueText(value, MaximumOperationIdBytes);

    private static bool IsOpaqueText(string value, int maximumBytes)
    {
        if (string.IsNullOrEmpty(value) || value.Length > maximumBytes)
            return false;
        foreach (char character in value)
        {
            if (!(character is >= 'a' and <= 'z' or >= 'A' and <= 'Z' or >= '0' and <= '9'
                or '-' or '_' or ':'))
                return false;
        }
        return true;
    }
}

internal enum OpaqueOperationNativeMessageKind : byte
{
    Invalid,
    Request,
    SettledReply,
    PendingReply,
    RejectedReply
}
