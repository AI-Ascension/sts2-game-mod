// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const uint RuntimeRequestKindGameplay = 6;
    private static RuntimeV3GameplaySupport? _runtimeV3Gameplay;
    private static RuntimeV3GameplayRecoveryStore? _runtimeV3RecoveryStore;

    private static void InitializeRuntimeV3Gameplay()
    {
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.Unconfigured();
    }

    private static void ConfigureRuntimeV3Gameplay(IRuntimeV3HostSource source, IRuntimeV3HostThread thread)
    {
        bool recoveryRequired = string.Equals(
            System.Environment.GetEnvironmentVariable("STS2_LIVE_COMBAT"), "1",
            StringComparison.Ordinal);
        string? directory = System.Environment.GetEnvironmentVariable("STS2_RUNTIME_RECOVERY_DIR");
        if (string.IsNullOrWhiteSpace(directory)
            || !TryReadSecret("STS2_RUNTIME_BOOTSTRAP_SECRET", out byte[]? bootstrap)
            || !TryReadSecret("STS2_RUNTIME_RECOVERY_READ_SECRET", out byte[]? historical)
            || !TryReadSecret("STS2_RUNTIME_RECOVERY_RECONCILE_SECRET", out byte[]? reconcile)
            || !RuntimeV3GameplayRecoveryStore.TryOpen(
                directory,
                new RuntimeV3RecoveryCredentials(bootstrap!, historical!, reconcile!),
                out RuntimeV3GameplayRecoveryStore? store,
                out _))
        {
            if (recoveryRequired)
            {
                Console.Error.WriteLine("[AI-ASCENSION STS2 GAME MOD] Runtime-v3 recovery storage is unavailable; "
                    + "mutation admission remains blocked");
            }
            _runtimeV3Gameplay = RuntimeV3GameplaySupport.WithHost(
                source, thread, () => _runtimeV2Pending is null,
                recoveryRequired: recoveryRequired);
            return;
        }

        _runtimeV3RecoveryStore = store;
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.WithHost(source, thread,
            () => _runtimeV2Pending is null, store, recoveryRequired);
    }

    private static bool TryReadSecret(string variable, out byte[]? secret)
    {
        secret = null;
        string? encoded = System.Environment.GetEnvironmentVariable(variable);
        if (string.IsNullOrWhiteSpace(encoded) || encoded.Length > 512)
        {
            return false;
        }
        try
        {
            byte[] decoded = Convert.FromBase64String(encoded);
            if (decoded.Length < 16)
            {
                return false;
            }
            secret = decoded;
            return true;
        }
        catch (FormatException)
        {
            return false;
        }
    }

    private static (int Status, string Response) ProcessRuntimeV3GameplayWork(
        RuntimeContext context,
        string body)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string error))
        {
            return (RuntimeRejected, RuntimeV2PlainError(error));
        }
        RuntimeV3GameplaySupport support = _runtimeV3Gameplay ?? RuntimeV3GameplaySupport.Unconfigured();
        string response = support.Handle(
            context.InstanceId,
            context.SessionId,
            context.LeaseId,
            context.CorrelationId,
            context.LeaseEpoch,
            body,
            out int status);
        return (status, response);
    }
}
