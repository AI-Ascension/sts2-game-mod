// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record BuildManifest(
    string GameBuild,
    string ModBuild,
    string ProtocolRevision,
    string SchemaDigest,
    string Source)
{
    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(GameBuild)
            || !RuntimeV3GameplayContract.IsIdentity(ModBuild)
            || !RuntimeV3GameplayContract.IsIdentity(ProtocolRevision)
            || SchemaDigest.Length != 64
            || Source.Length == 0)
        {
            error = "build manifest is incomplete or unsafe";
            return false;
        }
        error = string.Empty;
        return true;
    }

    internal IReadOnlyDictionary<string, string> ToSafeFields() =>
        new Dictionary<string, string>
        {
            ["game_build"] = GameBuild,
            ["mod_build"] = ModBuild,
            ["protocol_revision"] = ProtocolRevision,
            ["schema_digest"] = SchemaDigest,
            ["source"] = Source
        };
}
