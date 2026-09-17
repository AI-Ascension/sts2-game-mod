// SPDX-License-Identifier: MIT

using System.Runtime.CompilerServices;

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection.Tests;

/// <summary>
/// Opt-in regressions against the exact pinned host assembly. They fail closed without
/// <c>STS2GameDataDir</c> and are deliberately not part of hosted CI.
/// </summary>
internal static class CoverageInventoryTests
{
    private const string RunRngSet = "MegaCrit.Sts2.Core.Runs.RunRngSet";
    private static readonly string[] Adr0037Rows =
    [
        "Compatibility", "Seed and entropy", "Campaign", "Player", "Combat", "Non-combat decisions", "Persistence and identity",
    ];
    private static readonly string[] Adr0040Rows =
    [
        "Master seed", "Map/act generation", "Encounters/enemies/targeting", "Shuffle/draw/rewards/shops/events",
        "Ordering and external entropy", "Cosmetic-only randomness",
    ];

    internal static int Run(TextWriter output, TextWriter error)
    {
        string? directory = HostDataDirectory.Resolve(null);
        if (directory is null)
        {
            error.WriteLine("STS2GameDataDir is not set; these opt-in tests fail closed without the exact pinned host assembly.");
            return 2;
        }
        HostAssemblyPins pins = HostAssemblyPins.Read(directory);
        EveryInventoryFamilyResolvesToPinnedMember(pins);
        UnknownFamilyFailsClosed(pins);
        ReportsCarryNoHostValuesOrPaths(pins);
        ProbeSourceStaysMetadataOnly();
        output.WriteLine("checkpoint coverage reflection tests passed");
        return 0;
    }

    private static void EveryInventoryFamilyResolvesToPinnedMember(HostAssemblyPins pins)
    {
        Assert(pins.MatchesPinnedBuild, "supplied host does not match the pinned " + PinnedHostBuild.Version + " build");
        CoverageInventoryReport report = CoverageResolver.Resolve(pins, CoverageFamilies.All);
        Assert(report.Host.MatchesPinnedBuild && report.Host.Sts2Sha256 == PinnedHostBuild.Sts2Sha256, "report lost the host pin");
        Assert(report.Families.Count == CoverageFamilies.All.Count, "family count changed during resolution");
        Assert(report.Families.Select(family => family.Id).Distinct(StringComparer.Ordinal).Count() == report.Families.Count,
            "family ids are not unique");
        string[] unresolved = report.Families.Where(family => !family.IsResolved).Select(family => family.Id).ToArray();
        Assert(report.UnresolvedFamilyCount == 0 && unresolved.Length == 0, "unresolved families: " + string.Join(", ", unresolved));
        AssertRows(report, "0037", Adr0037Rows);
        AssertRows(report, "0040", Adr0040Rows);
        foreach (FamilyResolution family in report.Families)
        {
            Assert(family.Status == CoverageStatus.MetadataObserved, family.Id + " is not metadata-observed");
            Assert(family.Semantics == CoverageStatus.RuntimeUnverified, family.Id + " must stay runtime-unverified");
            Assert(family.Members.Count > 0, family.Id + " has no members");
            foreach (MemberResolution member in family.Members)
            {
                bool observed = member.Status == CoverageStatus.MetadataObserved;
                bool absent = member.Status == CoverageStatus.MetadataAbsent;
                Assert(observed || absent, family.Id + ": " + member.Type + "." + member.Member + " is " + member.Status + " (" + member.Detail + ")");
                Assert(!observed || member.Visibility is not null, family.Id + ": " + member.Member + " lacks visibility");
                Assert(!observed || member.Kind == "type" || member.DeclaredType is not null, family.Id + ": " + member.Member + " lacks a declared type");
                Assert(!absent || member.Kind == "no-member-containing", family.Id + ": absent status on a wrong kind");
            }
        }
    }

    private static void AssertRows(CoverageInventoryReport report, string adr, string[] expected)
    {
        string[] actual = report.Families.Where(family => family.Adr == adr).Select(family => family.Row).ToArray();
        Assert(actual.SequenceEqual(expected, StringComparer.Ordinal), "ADR " + adr + " rows differ: " + string.Join(", ", actual));
    }

    private static void UnknownFamilyFailsClosed(HostAssemblyPins pins)
    {
        var synthetic = new CoverageFamily("synthetic-unknown", "0037", "Synthetic", "n/a", "n/a", "n/a", "n/a",
        [
            new MemberExpectation("MegaCrit.Sts2.Core.Runs.NoSuchCheckpointType", "Seed", MemberKind.Property, "missing type"),
            new MemberExpectation(RunRngSet, "NoSuchCheckpointMember", MemberKind.Property, "missing member"),
            new MemberExpectation(RunRngSet, "StringSeed", MemberKind.Field, "property claimed as field"),
            new MemberExpectation(RunRngSet, "Seed", MemberKind.Method, "property claimed as method"),
            new MemberExpectation(RunRngSet, "stringseed", MemberKind.Property, "member name case"),
            new MemberExpectation("MegaCrit.Sts2.Core.runs.RunRngSet", "Seed", MemberKind.Property, "type name case"),
            new MemberExpectation("Runs.RunRngSet", "Seed", MemberKind.Property, "type name suffix"),
            new MemberExpectation(RunRngSet, "String", MemberKind.Property, "member name prefix"),
            new MemberExpectation(RunRngSet, "Seed", MemberKind.NoMemberContaining, "present text claimed absent"),
        ]);
        var control = new CoverageFamily("synthetic-control", "0037", "Control", "n/a", "n/a", "n/a", "n/a",
            [new MemberExpectation(RunRngSet, "StringSeed", MemberKind.Property, "control")]);
        var empty = new CoverageFamily("synthetic-empty", "0037", "Empty", "n/a", "n/a", "n/a", "n/a", []);

        CoverageInventoryReport report = CoverageResolver.Resolve(pins, [synthetic, control, empty]);
        FamilyResolution unknown = report.Families[0];
        Assert(unknown.Status == CoverageStatus.Unresolved, "the synthetic family did not fail closed");
        foreach (MemberResolution member in unknown.Members)
        {
            Assert(member.Status == CoverageStatus.Unresolved && member.DeclaredType is null && member.Visibility is null,
                "synthetic row resolved leniently: " + member.Item + " -> " + member.Status);
            Assert(!string.IsNullOrEmpty(member.Detail), "unresolved row lacks a reason: " + member.Item);
        }
        Assert(unknown.Members.Count == synthetic.Members.Count, "an unresolved row was dropped instead of reported");
        Assert(report.Families[1].Status == CoverageStatus.MetadataObserved, "the known control row did not resolve");
        Assert(report.Families[2].Status == CoverageStatus.Unresolved, "an empty family must not count as observed");
        Assert(report.UnresolvedFamilyCount == 2, "unresolved count must include the synthetic and empty families only");
        string markdown = CoverageMarkdown.Render(report, "0037", "t", "d", "c");
        Assert(markdown.Contains("| missing type | `Runs.NoSuchCheckpointType.Seed` | property | - | - | unresolved (", StringComparison.Ordinal),
            "markdown must label the unresolved row explicitly");
    }

    private static void ReportsCarryNoHostValuesOrPaths(HostAssemblyPins pins)
    {
        CoverageInventoryReport report = CoverageResolver.Resolve(pins, CoverageFamilies.All);
        string json = CoverageReportJson.Serialize(report);
        string markdown = CoverageMarkdown.Render(report, "0037", "t", "d", "c") + CoverageMarkdown.Render(report, "0040", "t", "d", "c");
        foreach (string text in new[] { json, markdown })
        {
            Assert(!text.Contains(pins.DataDirectory, StringComparison.OrdinalIgnoreCase), "report leaks the host data directory");
            Assert(!text.Contains(Path.GetFileName(pins.DataDirectory), StringComparison.OrdinalIgnoreCase), "report leaks the host directory name");
            Assert(text.Contains(CoverageStatus.Evidence, StringComparison.Ordinal), "report lost its evidence label");
            string withoutNegations = text.Replace("No row is native-verified", string.Empty, StringComparison.Ordinal)
                .Replace("Nothing here is native-verified", string.Empty, StringComparison.Ordinal);
            Assert(!withoutNegations.Contains("native-verified", StringComparison.Ordinal), "report must not claim native verification");
        }
        Assert(json.Contains("\"semantics\": \"runtime-unverified\"", StringComparison.Ordinal), "JSON lost the runtime-unverified label");
    }

    private static void ProbeSourceStaysMetadataOnly()
    {
        string probeDirectory = Path.GetFullPath(Path.Combine(TestDirectory(), "..", "checkpoint-coverage-reflection"));
        Assert(Directory.Exists(probeDirectory), "probe source directory not found: " + probeDirectory);
        string[] forbidden = ["Assembly.Load", "GetTypes(", "MetadataLoadContext", "AssemblyLoadContext", "Activator.CreateInstance", "FieldInfo", "PropertyInfo", "MethodInfo", ".Invoke("];
        int inspected = 0;
        foreach (string source in Directory.EnumerateFiles(probeDirectory, "*.cs"))
        {
            inspected++;
            string contents = File.ReadAllText(source);
            foreach (string api in forbidden)
                Assert(!contents.Contains(api, StringComparison.Ordinal), "unsafe reflection API " + api + " in " + Path.GetFileName(source));
        }
        Assert(inspected >= 8, "probe sources were not inspected");
    }

    private static string TestDirectory([CallerFilePath] string path = "") => Path.GetDirectoryName(path) ?? string.Empty;

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
