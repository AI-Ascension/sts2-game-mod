// SPDX-License-Identifier: MIT

using System.Reflection.Metadata;
using System.Reflection.PortableExecutable;

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

/// <summary>Resolves inventory families against one hashed host assembly through its metadata tables only.</summary>
internal static class CoverageResolver
{
    private const string EnumerationRule =
        "Every row names one exact host type and member; the full type name, member name, and member kind "
        + "match ordinally. A row that does not resolve is reported unresolved and is never substituted, "
        + "relaxed, or omitted, and a family with any unresolved row or no rows fails closed.";

    private const string Limitation =
        "Metadata proves that a member exists with a declared type on the hashed assembly. It cannot prove "
        + "serialization, restore responsibility, ordering, unknown-value handling, RNG consumption, thread "
        + "affinity, or that a capture is coherent; those stay runtime-unverified until an authorized exact-host "
        + "run records them. No row is native-verified.";

    internal static CoverageInventoryReport Resolve(HostAssemblyPins pins, IReadOnlyList<CoverageFamily> families)
    {
        using FileStream stream = new(pins.Sts2Path, FileMode.Open, FileAccess.Read, FileShare.Read);
        using var pe = new PEReader(stream, PEStreamOptions.LeaveOpen);
        if (!pe.HasMetadata) throw new BadImageFormatException("PE image has no metadata");
        var index = new MetadataTypeIndex(pe.GetMetadataReader());
        var resolved = new List<FamilyResolution>(families.Count);
        foreach (CoverageFamily family in families) resolved.Add(ResolveFamily(index, family));
        int unresolved = resolved.Count(family => !family.IsResolved);
        var host = new HostPinReport(index.AssemblyName, pins.Sts2Sha256, pins.GodotSharpSha256,
            PinnedHostBuild.Version, PinnedHostBuild.Commit, pins.MatchesPinnedBuild);
        return new CoverageInventoryReport(CoverageStatus.SchemaVersion, CoverageStatus.Evidence,
            CoverageStatus.RuntimeUnverified, host, resolved, unresolved, EnumerationRule, Limitation);
    }

    internal static FamilyResolution ResolveFamily(MetadataTypeIndex index, CoverageFamily family)
    {
        var members = new List<MemberResolution>(family.Members.Count);
        foreach (MemberExpectation expectation in family.Members) members.Add(index.Resolve(expectation));
        bool observed = members.Count > 0
            && members.All(member => !string.Equals(member.Status, CoverageStatus.Unresolved, StringComparison.Ordinal));
        return new FamilyResolution(family.Id, family.Adr, family.Row,
            observed ? CoverageStatus.MetadataObserved : CoverageStatus.Unresolved, CoverageStatus.RuntimeUnverified,
            family.Serialization, family.Ordering, family.Restore, family.UnknownPolicy, members);
    }
}
