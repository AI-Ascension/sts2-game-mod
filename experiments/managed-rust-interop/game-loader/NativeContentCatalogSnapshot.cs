// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Localization;
using MegaCrit.Sts2.Core.Modding;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Owner-thread capture of the verified host package and locale surfaces used by the #83
/// catalog adapter. This intentionally does not invent definition semantics or a generation
/// witness; callers must fail closed until the remaining typed source is supplied.
/// </summary>
internal sealed class NativeContentCatalogSnapshot
{
    internal sealed record Package(string PackageId, string? PackageVersion, uint Order);

    internal NativeContentCatalogSnapshot(string locale, IReadOnlyList<Package> packages)
    {
        Locale = locale;
        Packages = packages;
    }

    internal string Locale { get; }
    internal IReadOnlyList<Package> Packages { get; }

    internal static NativeContentCatalogSnapshot CaptureKnownInputs()
    {
        LocManager manager = LocManager.Instance
            ?? throw new InvalidOperationException("localization manager unavailable");
        string locale = manager.Language;
        if (!LookupBindingManifest.ValidLocale(locale))
            throw new InvalidOperationException("host locale is malformed");
        var packages = ModManager.Mods.Select((mod, order) =>
        {
            ModManifest manifest = mod.manifest
                ?? throw new InvalidOperationException("mod manifest unavailable");
            if (manifest.id is null
                || !RuntimeV3GameplayContract.IsIdentity(manifest.id)
                || (manifest.version is not null
                    && !RuntimeV3GameplayContract.IsIdentity(manifest.version)))
                throw new InvalidOperationException("mod manifest identity is malformed");
            return new Package(manifest.id, manifest.version, checked((uint)order));
        }).ToArray();
        return new NativeContentCatalogSnapshot(locale, packages);
    }
}
