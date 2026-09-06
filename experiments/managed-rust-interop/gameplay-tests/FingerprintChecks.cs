// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class FingerprintChecks
{
    internal static void Run()
    {
        var source = new FakeHost();
        var before = source.Observe();
        var actions = source.LegalActions(before);
        string fingerprint = RuntimeV3GameplayFingerprint.Create(before, actions);
        if (fingerprint != RuntimeV3GameplayFingerprint.Create(before with { }, actions))
            throw new InvalidOperationException("unchanged content must retain its fingerprint");
        foreach (var after in new[]
        {
            before with { IsActionable = !before.IsActionable },
            before with { InputEnabled = !before.InputEnabled },
            before with { ModalBlocking = !before.ModalBlocking },
            before with { TurnIndex = (ushort)(before.TurnIndex + 1) },
            before with { NodeId = "new-visible-node" }
        })
            if (fingerprint == RuntimeV3GameplayFingerprint.Create(after, actions))
                throw new InvalidOperationException("readiness and internal projection changes must be fenced");
        if (fingerprint == RuntimeV3GameplayFingerprint.Create(before, Array.Empty<LegalActionReference>()))
            throw new InvalidOperationException("catalog-only changes must be fenced");
    }
}
