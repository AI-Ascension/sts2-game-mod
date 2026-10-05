// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessHostPin
{
    internal const string HostVersion = "0.1.0.0";
    internal const string HarmonyVersion = "2.4.2.0";
    internal const string HostSha256 = "a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52";
    internal const string HarmonySha256 = "ef1898322c9f5c86dc1b0758b272a9c440823b4a41ca9a0b82a3aa6b3d206387";

    internal static bool IsSupported(Assembly hostAssembly, Assembly harmonyAssembly)
    {
        return HasIdentityAndHash(hostAssembly, HostVersion, HostSha256)
            && HasIdentityAndHash(harmonyAssembly, HarmonyVersion, HarmonySha256);
    }

    internal static bool MatchesIdentityAndHash(
        string? actualVersion,
        string? actualHash,
        string expectedVersion,
        string expectedHash) =>
        string.Equals(actualVersion, expectedVersion, StringComparison.Ordinal)
        && string.Equals(actualHash, expectedHash, StringComparison.OrdinalIgnoreCase);

    private static bool HasIdentityAndHash(Assembly assembly, string version, string expectedHash)
    {
        try
        {
            string? actualVersion = assembly.GetName().Version?.ToString();
            if (!string.Equals(actualVersion, version, StringComparison.Ordinal))
            {
                return false;
            }

            string location = assembly.Location;
            if (string.IsNullOrWhiteSpace(location))
            {
                return false;
            }

            using var stream = new FileStream(location, FileMode.Open, FileAccess.Read, FileShare.Read);
            byte[] digest = SHA256.HashData(stream);
            string actualHash = Convert.ToHexString(digest);
            return MatchesIdentityAndHash(
                actualVersion,
                actualHash,
                version,
                expectedHash);
        }
        catch (Exception)
        {
            return false;
        }
    }
}
