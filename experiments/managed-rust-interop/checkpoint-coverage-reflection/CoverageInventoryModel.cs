// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

/// <summary>Kind of host member an inventory row expects to find on the pinned assembly.</summary>
internal enum MemberKind
{
    Type,
    Field,
    Property,
    Method,

    /// <summary>
    /// Claims that no field, property, or method of the type contains the member text
    /// (ordinal, case-insensitive). Used to record that a coverage item has no host member.
    /// </summary>
    NoMemberContaining,
}

internal static class CoverageStatus
{
    internal const string SchemaVersion = "checkpoint-coverage-inventory-v1";
    internal const string Evidence = "metadata_only_no_target_assembly_load_or_execution";

    /// <summary>The named member exists on the hashed assembly with the recorded kind and declared type.</summary>
    internal const string MetadataObserved = "metadata-observed";

    /// <summary>No member of the named type contains the text; the coverage item has no host member.</summary>
    internal const string MetadataAbsent = "metadata-absent";

    /// <summary>The row could not be matched exactly; the family fails closed.</summary>
    internal const string Unresolved = "unresolved";

    /// <summary>Serialization, ordering, restore, and unknown-value semantics need a live exact host.</summary>
    internal const string RuntimeUnverified = "runtime-unverified";
}

/// <summary>One expected host member. <see cref="Item"/> names the ADR closure item it serves.</summary>
internal sealed record MemberExpectation(string Type, string Member, MemberKind Kind, string Item);

/// <summary>One ADR 0037 coverage family or ADR 0040 audit row with its policy statements.</summary>
internal sealed record CoverageFamily(string Id, string Adr, string Row, string Serialization, string Ordering,
    string Restore, string UnknownPolicy, IReadOnlyList<MemberExpectation> Members);

internal sealed record MemberResolution(string Item, string Type, string Member, string Kind, string Status,
    string? DeclaredType, string? Visibility, string? Detail);

internal sealed record FamilyResolution(string Id, string Adr, string Row, string Status, string Semantics,
    string Serialization, string Ordering, string Restore, string UnknownPolicy,
    IReadOnlyList<MemberResolution> Members)
{
    internal bool IsResolved => !string.Equals(Status, CoverageStatus.Unresolved, StringComparison.Ordinal);
}

/// <summary>Identifies the inspected host by hash only; no path, byte, or value is recorded.</summary>
internal sealed record HostPinReport(string AssemblyName, string Sts2Sha256, string? GodotSharpSha256,
    string PinnedVersion, string PinnedCommit, bool MatchesPinnedBuild);

internal sealed record CoverageInventoryReport(string SchemaVersion, string Evidence, string Semantics,
    HostPinReport Host, IReadOnlyList<FamilyResolution> Families, int UnresolvedFamilyCount,
    string EnumerationRule, string Limitation);
