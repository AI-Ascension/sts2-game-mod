// SPDX-License-Identifier: MIT

using System;
using System.Diagnostics;
using System.IO;
using System.Security.Cryptography;
using System.Threading;
using System.Threading.Tasks;

namespace AiAscension.Sts2ModelDbRegistryProbe;

internal static class HostBinaryHashVerifier
{
    internal const long MaxAssemblyBytes = 128L * 1024 * 1024;
    private const int BufferBytes = 64 * 1024;

    internal static async Task<string> VerifyAsync(
        string loadedAssemblyPath,
        string expectedSha256,
        TimeSpan timeout,
        CancellationToken cancellationToken,
        long maxAssemblyBytes = MaxAssemblyBytes)
    {
        if (string.IsNullOrWhiteSpace(loadedAssemblyPath)
            || expectedSha256.Length != 64
            || timeout <= TimeSpan.Zero
            || maxAssemblyBytes <= 0)
        {
            throw new ProbeFailure("invalid_binary_preflight");
        }

        using var timeoutSource = new CancellationTokenSource(timeout);
        using var linkedSource = CancellationTokenSource.CreateLinkedTokenSource(
            cancellationToken, timeoutSource.Token);
        CancellationToken token = linkedSource.Token;

        try
        {
            var file = new FileInfo(loadedAssemblyPath);
            if (!file.Exists || file.Length <= 0)
                throw new ProbeFailure("host_binary_unavailable");
            if (file.Length > maxAssemblyBytes)
                throw new ProbeFailure("host_binary_limit_exceeded");

            using var stream = new FileStream(
                loadedAssemblyPath,
                FileMode.Open,
                FileAccess.Read,
                FileShare.Read,
                BufferBytes,
                FileOptions.Asynchronous | FileOptions.SequentialScan);
            using IncrementalHash hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
            byte[] buffer = new byte[BufferBytes];
            long totalBytes = 0;
            var elapsed = Stopwatch.StartNew();
            int read;
            while ((read = await stream.ReadAsync(buffer.AsMemory(), token).ConfigureAwait(false)) != 0)
            {
                token.ThrowIfCancellationRequested();
                totalBytes = checked(totalBytes + read);
                if (totalBytes > maxAssemblyBytes)
                    throw new ProbeFailure("host_binary_limit_exceeded");
                if (elapsed.Elapsed >= timeout)
                    throw new ProbeFailure("host_binary_timeout");
                hash.AppendData(buffer, 0, read);
            }

            token.ThrowIfCancellationRequested();
            if (elapsed.Elapsed >= timeout)
                throw new ProbeFailure("host_binary_timeout");
            byte[] actual = hash.GetHashAndReset();
            byte[] expected;
            try
            {
                expected = Convert.FromHexString(expectedSha256);
            }
            catch (FormatException)
            {
                throw new ProbeFailure("invalid_binary_preflight");
            }

            if (!CryptographicOperations.FixedTimeEquals(actual, expected))
                throw new ProbeFailure("host_binary_pin_mismatch");
            return Convert.ToHexString(actual).ToLowerInvariant();
        }
        catch (OperationCanceledException)
        {
            if (cancellationToken.IsCancellationRequested)
                throw new ProbeFailure("probe_cancelled");
            throw new ProbeFailure("host_binary_timeout");
        }
        catch (ProbeFailure)
        {
            throw;
        }
        catch (Exception)
        {
            throw new ProbeFailure("host_binary_unavailable");
        }
    }
}
