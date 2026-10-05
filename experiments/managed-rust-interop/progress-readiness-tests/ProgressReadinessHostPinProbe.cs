// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessHostPinProbe
{
    internal static int Run()
    {
        const string measuredHostHash = "a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52";
        const string transposedHostHash = "a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62d6c7a52";
        Assert(ProgressReadinessHostPin.MatchesIdentityAndHash(
            ProgressReadinessHostPin.HostVersion,
            measuredHostHash,
            ProgressReadinessHostPin.HostVersion,
            ProgressReadinessHostPin.HostSha256), "measured host digest matches production pin");
        Assert(!ProgressReadinessHostPin.MatchesIdentityAndHash(
            ProgressReadinessHostPin.HostVersion,
            transposedHostHash,
            ProgressReadinessHostPin.HostVersion,
            ProgressReadinessHostPin.HostSha256), "transposed host digest is refused");
        Assert(!ProgressReadinessHostPin.MatchesIdentityAndHash(
            "0.1.0.1",
            measuredHostHash,
            ProgressReadinessHostPin.HostVersion,
            ProgressReadinessHostPin.HostSha256), "wrong host version is refused");
        return 3;
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException($"failed: {message}");
        }
    }
}
