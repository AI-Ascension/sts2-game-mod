// SPDX-License-Identifier: MIT

using System;
using System.Reflection;
using AiAscension.Sts2GameMod.Runtime;

internal static class ProgressReadinessPatchBindingProbe
{
    internal static int Run()
    {
        MethodInfo expectedPatch = Method(nameof(ExpectedPatch));
        MethodInfo wrongPatch = Method(nameof(WrongPatch));
        Assert(true, new[] { Owned(expectedPatch) }, expectedPatch, "exact owned patch accepted");
        Assert(false, new[] { Owned(wrongPatch) }, expectedPatch, "wrong same-owner method refused");
        Assert(false, new[] { Owned(expectedPatch), Owned(wrongPatch) }, expectedPatch, "duplicate owner patches refused");
        Assert(true, new[] { new ProgressReadinessPatchBinding("independent.addon.owner", wrongPatch), Owned(expectedPatch) }, expectedPatch, "other owner remains untouched");
        return 4;
    }

    private static ProgressReadinessPatchBinding Owned(MethodInfo method) =>
        new(ProgressReadinessPatchRegistration.OwnerId, method);

    private static MethodInfo Method(string name) =>
        typeof(ProgressReadinessPatchBindingProbe).GetMethod(name, BindingFlags.Static | BindingFlags.NonPublic)!;

    private static void Assert(
        bool expected,
        ProgressReadinessPatchBinding[] bindings,
        MethodInfo method,
        string label)
    {
        bool actual = ProgressReadinessPatchVerifier.HasExactlyOneOwnedMethod(
            bindings,
            ProgressReadinessPatchRegistration.OwnerId,
            method);
        if (actual != expected)
        {
            throw new InvalidOperationException($"failed: {label}");
        }
    }

    private static void ExpectedPatch() { }

    private static void WrongPatch() { }
}
