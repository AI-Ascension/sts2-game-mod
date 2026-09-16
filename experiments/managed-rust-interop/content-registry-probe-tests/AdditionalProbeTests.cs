// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Security.Cryptography;
using System.Threading;
using System.Threading.Tasks;
using AiAscension.Sts2ModelDbRegistryProbe;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2ModelDbRegistryProbe.Tests;

internal static partial class Program
{
    private static void TestMutationAndResolverFailuresRefuse()
    {
        ResetRegistry();
        ModelDb.AddForTest("card", "one", new SyntheticCard());
        ModelDb.AddForTest("relic", "two", new SyntheticRelic());
        bool mutated = false;
        ExpectFailure("registry_changed", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(), type =>
            {
                if (!mutated)
                {
                    mutated = true;
                    ModelDb.AddForTest("card", "added", new SyntheticCard());
                }
                return ModelDb.GetCategoryType(type);
            }, ModelDbRegistryCapture.ExpectedGameBuild, Environment.CurrentManagedThreadId,
                CancellationToken.None, NormalLimits));

        ResetRegistry();
        ModelDb.AddForTest("card", "one", new SyntheticCard());
        ExpectFailure("registry_read_failed", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                _ => throw new InvalidOperationException("private diagnostic detail"),
                ModelDbRegistryCapture.ExpectedGameBuild, Environment.CurrentManagedThreadId,
                CancellationToken.None, NormalLimits));

        ResetRegistry();
        ModelDb.AddForTest("card", "one", new SyntheticCard());
        using var cancellation = new CancellationTokenSource();
        ExpectFailure("probe_cancelled", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(), type =>
            {
                cancellation.Cancel();
                return ModelDb.GetCategoryType(type);
            }, ModelDbRegistryCapture.ExpectedGameBuild, Environment.CurrentManagedThreadId,
                cancellation.Token, NormalLimits));
    }

    private static void TestBoundedReport()
    {
        ResetRegistry();
        ModelDb.AddForTest("card", "one", new SyntheticCard());
        RegistryProbeSnapshot snapshot = Capture();
        string report = snapshot.SerializeBoundedReport(new string('a', 64), 4096);
        Check(report.Contains("\"game_build\":\"v0.107.1\"", StringComparison.Ordinal)
            && report.Contains("\"items\"", StringComparison.Ordinal),
            "serializes a bounded owned report");
        ExpectFailure("report_limit_exceeded", () =>
            snapshot.SerializeBoundedReport(new string('a', 64), 1));
    }

    private static async Task TestBinaryHashPreflight()
    {
        string path = Path.Combine(Path.GetTempPath(), "modeldb-probe-" + Guid.NewGuid().ToString("N"));
        byte[] bytes = [1, 2, 3, 4, 5, 6, 7];
        try
        {
            await File.WriteAllBytesAsync(path, bytes);
            string expected = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
            string actual = await HostBinaryHashVerifier.VerifyAsync(
                path, expected, TimeSpan.FromSeconds(2), CancellationToken.None);
            Check(actual == expected, "verifies the selected host binary hash");
            await ExpectFailureAsync("host_binary_limit_exceeded", async () =>
                await HostBinaryHashVerifier.VerifyAsync(
                    path, expected, TimeSpan.FromSeconds(2), CancellationToken.None,
                    maxAssemblyBytes: bytes.Length - 1));
            await ExpectFailureAsync("host_binary_pin_mismatch", async () =>
                await HostBinaryHashVerifier.VerifyAsync(
                    path, new string('0', 64), TimeSpan.FromSeconds(2), CancellationToken.None));
            using var canceled = new CancellationTokenSource();
            canceled.Cancel();
            await ExpectFailureAsync("probe_cancelled", async () =>
                await HostBinaryHashVerifier.VerifyAsync(
                    path, expected, TimeSpan.FromSeconds(2), canceled.Token));
        }
        finally
        {
            if (File.Exists(path))
                File.Delete(path);
        }
    }

    private static async Task ExpectFailureAsync(string code, Func<Task> action)
    {
        try
        {
            await action();
        }
        catch (ProbeFailure failure)
        {
            Check(failure.Code == code, $"refuses with sanitized {code}");
            return;
        }
        throw new InvalidOperationException("expected failure " + code);
    }
}
