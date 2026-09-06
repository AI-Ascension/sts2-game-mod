// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Constants and authenticated proof helpers for the additive recovery sideband. Runtime-v3
/// gameplay messages continue to use <see cref="RuntimeV3GameplayContract"/> unchanged.
/// </summary>
internal static class RuntimeV3GameplayRecoveryContract
{
    internal const string Contract = "watchdog-recovery-v1";
    internal const string SchemaDigest =
        "fb934d3157485aaf6e13e6ebbb213ec8a14c7fc6f5eeebc06b7a22c1f0009217";
    internal const string Canonicalization = "RCJ-1";
    internal const int MaxFrameBytes = 262_144;
    internal const int MaxActionBytes = 65_536;
    internal const int MaxJournalBytes = 256 * 1024 * 1024;
    internal const int MaxOperationRecords = 16_384;
    internal const ulong MaxWireInteger = 9_007_199_254_740_991;

    internal static bool IsDigest(string value) =>
        value is not null && value.Length == 64 && value.All(static character =>
            character is >= '0' and <= '9' or >= 'a' and <= 'f');

    internal static bool IsUuid(string value) =>
        value is not null
        && Guid.TryParseExact(value, "D", out Guid parsed)
        && string.Equals(parsed.ToString("D"), value, StringComparison.Ordinal);

    internal static bool IsRandomUuid(string value) =>
        IsUuid(value) && value[14] == '4' && value[19] is '8' or '9' or 'a' or 'b';

    internal static bool IsTimestamp(DateTimeOffset value) =>
        value.Offset == TimeSpan.Zero && value.Year is >= 2000 and <= 9999;

    internal static string Digest(ReadOnlySpan<byte> bytes) =>
        Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();

    internal static string CreateBootstrapProof(
        byte[] secret, RuntimeV3HostFence fence, RuntimeV3RecoveryRelease release) =>
        CreateProof(secret, "bootstrap", FenceBytes(fence, release));

    internal static string CreateHistoricalReadProof(
        byte[] secret, RuntimeV3OperationKey operation, string payloadDigest) =>
        CreateProof(secret, "historical-read", OperationBytes(operation, payloadDigest));

    internal static string CreateReconcileProof(
        byte[] secret, RuntimeV3OperationKey operation, string payloadDigest) =>
        CreateProof(secret, "reconcile", OperationBytes(operation, payloadDigest));

    internal static bool VerifyProof(byte[] secret, string purpose, string bytes, string proof)
    {
        if (string.IsNullOrEmpty(proof) || proof.Length > 512)
        {
            return false;
        }

        byte[] expected = HMACSHA256.HashData(secret, Encoding.UTF8.GetBytes($"{purpose}\n{bytes}"));
        byte[] supplied;
        try
        {
            supplied = Convert.FromBase64String(proof.Replace('-', '+').Replace('_', '/')
                + new string('=', (4 - proof.Length % 4) % 4));
        }
        catch (FormatException)
        {
            return false;
        }

        return CryptographicOperations.FixedTimeEquals(expected, supplied);
    }

    private static string CreateProof(byte[] secret, string purpose, string bytes)
    {
        byte[] digest = HMACSHA256.HashData(secret, Encoding.UTF8.GetBytes($"{purpose}\n{bytes}"));
        return Convert.ToBase64String(digest).TrimEnd('=').Replace('+', '-').Replace('/', '_');
    }

    private static string FenceBytes(RuntimeV3HostFence fence, RuntimeV3RecoveryRelease release) =>
        string.Join("|", Contract, SchemaDigest, RuntimeV3GameplayContract.SchemaDigest,
            fence.DeploymentId, fence.InstanceId, fence.InstanceIncarnation, fence.BootId,
            fence.AuthorityGeneration, fence.LeaseId, fence.LeaseEpoch, fence.HostFenceId,
            fence.FenceGeneration, fence.ExpiresAt.UtcDateTime.ToString("O"),
            release.ReleaseDigest, release.ConfigDigest, release.ProfileDigest,
            release.RuntimeV3SchemaDigest);

    private static string OperationBytes(RuntimeV3OperationKey operation, string payloadDigest) =>
        string.Join("|", operation.InstanceId, operation.SessionId, operation.LeaseId,
            operation.LeaseEpoch, operation.OperationId, payloadDigest);
}

internal sealed class RuntimeV3RecoveryCredentials
{
    internal RuntimeV3RecoveryCredentials(byte[] bootstrapSecret, byte[] historicalReadSecret,
        byte[] reconcileSecret)
    {
        if (bootstrapSecret.Length < 16 || historicalReadSecret.Length < 16 || reconcileSecret.Length < 16)
        {
            throw new ArgumentException("recovery credentials must contain at least 16 bytes each");
        }

        BootstrapSecret = (byte[])bootstrapSecret.Clone();
        HistoricalReadSecret = (byte[])historicalReadSecret.Clone();
        ReconcileSecret = (byte[])reconcileSecret.Clone();
    }

    internal byte[] BootstrapSecret { get; }
    internal byte[] HistoricalReadSecret { get; }
    internal byte[] ReconcileSecret { get; }
}
