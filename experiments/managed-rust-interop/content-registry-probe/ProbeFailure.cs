// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2ModelDbRegistryProbe;

internal sealed class ProbeFailure : Exception
{
    internal ProbeFailure(string code) : base(code)
    {
        Code = code;
    }

    internal string Code { get; }
}
