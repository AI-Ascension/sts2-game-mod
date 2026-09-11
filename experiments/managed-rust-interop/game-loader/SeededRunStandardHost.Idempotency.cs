// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class SeededRunStandardHost
{
    internal static bool IsExactReplay(
        string retainedFingerprint,
        string requestFingerprint,
        ulong retainedRequestGeneration,
        ulong requestGeneration) =>
        string.Equals(retainedFingerprint, requestFingerprint, StringComparison.Ordinal)
        && retainedRequestGeneration == requestGeneration;
}
