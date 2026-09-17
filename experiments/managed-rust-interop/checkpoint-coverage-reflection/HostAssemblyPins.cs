// SPDX-License-Identifier: MIT

using System.Reflection;
using System.Security.Cryptography;

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

/// <summary>
/// The exact host build this inventory is recorded against (ADR 0002 pin). Hashes identify the
/// operator-supplied files; the probe never copies, loads, or executes them.
/// </summary>
internal static class PinnedHostBuild
{
    internal const string Version = "v0.107.1";
    internal const string Commit = "59260271";
    internal const string Sts2Sha256 = "a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52";
    internal const string GodotSharpSha256 = "0e4897ecdfb31456a97c7d8028dfb8d7dbdc632e2f73fc9b438d7b266a139289";
}

internal sealed record HostAssemblyPins(string DataDirectory, string Sts2Path, string Sts2Sha256, string? GodotSharpSha256)
{
    internal bool MatchesPinnedBuild =>
        string.Equals(Sts2Sha256, PinnedHostBuild.Sts2Sha256, StringComparison.Ordinal)
        && string.Equals(GodotSharpSha256, PinnedHostBuild.GodotSharpSha256, StringComparison.Ordinal);

    /// <summary>Hashes the host assemblies in place. Throws <see cref="FileNotFoundException"/> without <c>sts2.dll</c>.</summary>
    internal static HostAssemblyPins Read(string dataDirectory)
    {
        string root = Path.GetFullPath(dataDirectory);
        string sts2 = Path.Combine(root, "sts2.dll");
        if (!File.Exists(sts2))
            throw new FileNotFoundException("sts2.dll does not exist in the supplied data directory", sts2);
        string godotSharp = Path.Combine(root, "GodotSharp.dll");
        return new HostAssemblyPins(root, sts2, Sha256Hex(sts2), File.Exists(godotSharp) ? Sha256Hex(godotSharp) : null);
    }

    private static string Sha256Hex(string path)
    {
        using FileStream stream = new(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        return Convert.ToHexStringLower(SHA256.HashData(stream));
    }
}

/// <summary>Resolves the operator-supplied host data directory; there is no default.</summary>
internal static class HostDataDirectory
{
    internal const string PropertyName = "STS2GameDataDir";

    /// <summary>Explicit argument first, then the environment, then the value recorded at build time.</summary>
    internal static string? Resolve(string? explicitDirectory)
    {
        if (!string.IsNullOrWhiteSpace(explicitDirectory)) return explicitDirectory;
        string? fromEnvironment = Environment.GetEnvironmentVariable(PropertyName);
        if (!string.IsNullOrWhiteSpace(fromEnvironment)) return fromEnvironment;
        string? fromBuild = typeof(HostDataDirectory).Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .FirstOrDefault(attribute => string.Equals(attribute.Key, PropertyName, StringComparison.Ordinal))?.Value;
        return string.IsNullOrWhiteSpace(fromBuild) ? null : fromBuild;
    }
}
