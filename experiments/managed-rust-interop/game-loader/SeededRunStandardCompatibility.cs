// SPDX-License-Identifier: MIT

using System;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;
using MegaCrit.Sts2.Core.Nodes;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record SeededRunStandardCompatibilitySnapshot(
    string GameIdentity,
    string GameDigest,
    string ModIdentity,
    string ModDigest)
{
    internal string CombinedIdentity => $"game/{GameIdentity}/mod/{ModIdentity}";

    internal static SeededRunStandardCompatibilitySnapshot Current()
    {
        Assembly game = typeof(NGame).Assembly;
        Assembly mod = typeof(ModEntry).Assembly;
        string gameIdentity = AssemblyIdentity(game);
        string modIdentity = AssemblyIdentity(mod);
        return new SeededRunStandardCompatibilitySnapshot(
            gameIdentity,
            AssemblyDigest(game),
            modIdentity,
            AssemblyDigest(mod));
    }

    private static string AssemblyIdentity(Assembly assembly)
    {
        AssemblyName name = assembly.GetName();
        string simpleName = name.Name ?? throw new InvalidOperationException("assembly name unavailable");
        string version = name.Version?.ToString() ?? throw new InvalidOperationException("assembly version unavailable");
        string identity = $"{simpleName}/{version}";
        if (!SeededRunStandardContract.IsIdentity(identity))
        {
            throw new InvalidOperationException("assembly identity is outside the seeded-run identity grammar");
        }

        return identity;
    }

    private static string AssemblyDigest(Assembly assembly)
    {
        string location = assembly.Location;
        if (string.IsNullOrWhiteSpace(location) || !File.Exists(location))
        {
            throw new InvalidOperationException("assembly file location unavailable");
        }

        using FileStream stream = File.OpenRead(location);
        return Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }
}
