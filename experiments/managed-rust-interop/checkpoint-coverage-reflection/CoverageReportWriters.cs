// SPDX-License-Identifier: MIT

using System.Globalization;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

internal static class CoverageReportJson
{
    private static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
    };

    internal static string Serialize(CoverageInventoryReport report) => JsonSerializer.Serialize(report, Options);
}

/// <summary>Renders the families of one ADR as a review-friendly table. Host names are shortened for width only.</summary>
internal static class CoverageMarkdown
{
    private static readonly string[] Prefixes = ["MegaCrit.Sts2.Core.", "System.Collections.Generic.", "System."];

    internal static string Render(CoverageInventoryReport report, string adr, string title, string recordedOn, string command)
    {
        FamilyResolution[] families = report.Families
            .Where(family => string.Equals(family.Adr, adr, StringComparison.Ordinal)).ToArray();
        int unresolved = families.Count(family => !family.IsResolved);
        var text = new StringBuilder();
        text.Append("# ").Append(title).Append("\n\n");
        text.Append("Recorded on ").Append(recordedOn)
            .Append(" by `experiments/managed-rust-interop/checkpoint-coverage-reflection/` from the metadata tables of the\n")
            .Append("exact pinned host assembly. The probe never loads, copies, or executes the assembly and records member\n")
            .Append("names, kinds, and declared types only; no host value, byte, IL, string constant, or install path appears here.\n\n");
        text.Append("| Field | Value |\n| --- | --- |\n");
        Row(text, "Host build", report.Host.PinnedVersion + " / `" + report.Host.PinnedCommit + "` (pin match: "
            + (report.Host.MatchesPinnedBuild ? "yes" : "NO") + ")");
        Row(text, "`sts2.dll` SHA-256", "`" + report.Host.Sts2Sha256 + "`");
        Row(text, "`GodotSharp.dll` SHA-256", report.Host.GodotSharpSha256 is null ? "absent" : "`" + report.Host.GodotSharpSha256 + "`");
        Row(text, "Assembly name", "`" + report.Host.AssemblyName + "`");
        Row(text, "Evidence", "`" + report.Evidence + "`");
        Row(text, "Command", "`" + command + "`");
        Row(text, "Families", families.Length.ToString(CultureInfo.InvariantCulture) + " in this table; "
            + unresolved.ToString(CultureInfo.InvariantCulture) + " unresolved");
        text.Append('\n')
            .Append("Labels: `metadata-observed` means the named member exists on the hashed assembly with the recorded kind and\n")
            .Append("declared type; `metadata-absent` means no member of the named type contains the text; `unresolved` means the\n")
            .Append("row could not be matched exactly and its family fails closed. Serialization, ordering, restore, and\n")
            .Append("unknown-value statements below are policy derived from the observed shapes and stay `runtime-unverified`\n")
            .Append("until an authorized exact-host run records them. Nothing here is native-verified.\n");
        foreach (FamilyResolution family in families) AppendFamily(text, family);
        return text.ToString();
    }

    private static void AppendFamily(StringBuilder text, FamilyResolution family)
    {
        text.Append("\n## ").Append(family.Row).Append(" (`").Append(family.Id).Append("`)\n\n");
        text.Append("- Metadata status: `").Append(family.Status).Append("`; semantics: `").Append(family.Semantics).Append("`\n");
        text.Append("- Serialization: ").Append(family.Serialization).Append('\n');
        text.Append("- Ordering: ").Append(family.Ordering).Append('\n');
        text.Append("- Restore responsibility: ").Append(family.Restore).Append('\n');
        text.Append("- Unknown-value policy: ").Append(family.UnknownPolicy).Append("\n\n");
        text.Append("| Closure item | Host member | Kind | Declared type | Visibility | Status |\n");
        text.Append("| --- | --- | --- | --- | --- | --- |\n");
        foreach (MemberResolution member in family.Members)
        {
            string name = string.Equals(member.Kind, "type", StringComparison.Ordinal)
                ? Short(member.Type) : Short(member.Type) + "." + member.Member;
            text.Append("| ").Append(member.Item).Append(" | `").Append(name).Append("` | ").Append(member.Kind)
                .Append(" | ").Append(member.DeclaredType is null ? "-" : "`" + Short(member.DeclaredType) + "`")
                .Append(" | ").Append(member.Visibility ?? "-").Append(" | ").Append(member.Status)
                .Append(member.Detail is null ? string.Empty : " (" + member.Detail + ")").Append(" |\n");
        }
    }

    private static void Row(StringBuilder text, string field, string value) =>
        text.Append("| ").Append(field).Append(" | ").Append(value).Append(" |\n");

    private static string Short(string value)
    {
        foreach (string prefix in Prefixes) value = value.Replace(prefix, string.Empty, StringComparison.Ordinal);
        var result = new StringBuilder(value.Length);
        for (int index = 0; index < value.Length; index++)
        {
            if (value[index] == '`')
            {
                while (index + 1 < value.Length && char.IsAsciiDigit(value[index + 1])) index++;
                continue;
            }
            result.Append(value[index]);
        }
        return result.ToString();
    }
}
