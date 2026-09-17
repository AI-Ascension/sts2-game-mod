// SPDX-License-Identifier: MIT

using System.Globalization;

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

internal static class CoverageInventoryCli
{
    internal const string Usage = "usage: CheckpointCoverageReflection [--game-data-dir <dir>] [--output-dir <dir>] "
        + "[--output-stem <name>] [--recorded-on <date>]";

    private const string ProjectPath = "experiments/managed-rust-interop/checkpoint-coverage-reflection/CheckpointCoverageReflection.csproj";

    internal static int Execute(string[] args, TextWriter output, TextWriter error)
    {
        string? dataDirectory = null;
        string? outputDirectory = null;
        string stem = "checkpoint-coverage-inventory";
        string recordedOn = "an unspecified date";
        for (int index = 0; index < args.Length; index++)
        {
            string? value = index + 1 < args.Length ? args[index + 1] : null;
            switch (args[index])
            {
                case "--game-data-dir" when value is not null: dataDirectory = value; index++; break;
                case "--output-dir" when value is not null: outputDirectory = value; index++; break;
                case "--output-stem" when value is not null: stem = value; index++; break;
                case "--recorded-on" when value is not null: recordedOn = value; index++; break;
                default: error.WriteLine(Usage); return 2;
            }
        }

        string? resolvedDirectory = HostDataDirectory.Resolve(dataDirectory);
        if (resolvedDirectory is null)
        {
            error.WriteLine("STS2GameDataDir is not set. Pass --game-data-dir, export STS2GameDataDir, or build with "
                + "-p:STS2GameDataDir=<dir>. This opt-in probe fails closed without the exact pinned host assembly.");
            return 2;
        }

        HostAssemblyPins pins;
        try { pins = HostAssemblyPins.Read(resolvedDirectory); }
        catch (FileNotFoundException)
        {
            error.WriteLine("sts2.dll does not exist in the supplied data directory");
            return 2;
        }
        catch (IOException)
        {
            error.WriteLine("host assembly could not be hashed");
            return 1;
        }
        catch (UnauthorizedAccessException)
        {
            error.WriteLine("host assembly is not readable");
            return 1;
        }

        CoverageInventoryReport report;
        try { report = CoverageResolver.Resolve(pins, CoverageFamilies.All); }
        catch (BadImageFormatException)
        {
            error.WriteLine("sts2.dll has no readable managed metadata");
            return 1;
        }
        catch (IOException)
        {
            error.WriteLine("sts2.dll metadata could not be read");
            return 1;
        }

        string command = "DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 dotnet run --project " + ProjectPath
            + " -c Release -p:STS2GameDataDir=<operator-supplied-host-data> -p:ManagedBuildRoot=<external-build-root> -- "
            + "--output-dir <dir> --output-stem " + stem + " --recorded-on " + recordedOn;
        if (outputDirectory is null)
        {
            output.WriteLine(CoverageReportJson.Serialize(report));
        }
        else
        {
            Directory.CreateDirectory(outputDirectory);
            File.WriteAllText(Path.Combine(outputDirectory, stem + ".json"), CoverageReportJson.Serialize(report) + "\n");
            File.WriteAllText(Path.Combine(outputDirectory, stem + ".md"), CoverageMarkdown.Render(report, "0037",
                "Exact-build checkpoint coverage inventory (ADR 0037, pinned metadata)", recordedOn, command));
            File.WriteAllText(Path.Combine(outputDirectory, stem + "-rng.md"), CoverageMarkdown.Render(report, "0040",
                "Exact-build seed and RNG stream inventory (ADR 0040, pinned metadata)", recordedOn, command));
            output.WriteLine("checkpoint coverage inventory: " + report.Families.Count.ToString(CultureInfo.InvariantCulture)
                + " families, " + report.UnresolvedFamilyCount.ToString(CultureInfo.InvariantCulture)
                + " unresolved; sts2.dll sha256 " + report.Host.Sts2Sha256 + "; pinned build match: "
                + (report.Host.MatchesPinnedBuild ? "yes" : "no") + "; wrote " + stem + ".json, " + stem + ".md, "
                + stem + "-rng.md");
        }
        if (!report.Host.MatchesPinnedBuild)
            error.WriteLine("warning: the supplied host does not match the pinned " + PinnedHostBuild.Version
                + " build; rows describe the hashed assembly above, not the pinned one");
        return report.UnresolvedFamilyCount == 0 ? 0 : 1;
    }
}
