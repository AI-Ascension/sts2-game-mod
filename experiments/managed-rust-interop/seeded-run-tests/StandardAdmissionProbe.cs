// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class StandardAdmissionProbe
{
    private static readonly string[] Acts = { "act_1", "act_2", "act_3", "act_4" };
    private static readonly string[] Modifier = { "modifier" };

    private const string Digest =
        "4581aaf95348126550cdf3b73ec46b39d447523cf7cb35aec71c2842d1945031";

    private static void Main()
    {
        SeededRunSelectionContext supported = Context();
        Check(SeededRunStandardAdmission.HasSupportedStaticContext(supported, out _),
            "fresh, saving-enabled ascension-zero standard context is admitted");
        Check(Rejected(supported with
        {
            ProfileBaseline = new SeededRunContextProfileBaseline("existing", "fresh-profile", Digest)
        }), "existing profile context is rejected before host access");
        Check(Rejected(supported with { SavePolicy = "disabled" }),
            "save-disabled context is rejected before host access");
        Check(Rejected(supported with { SelectionPolicy = "manual" }),
            "non-standard selection policy is rejected before host access");
        Check(Rejected(supported with { Ascension = 1 }),
            "nonzero ascension is rejected before host access");
        Check(Rejected(supported with { Modifiers = Modifier }),
            "modifier-bearing context is rejected before host access");
        Console.WriteLine("seeded standard admission checks passed");
    }

    private static SeededRunSelectionContext Context() => new(
        "standard/ironclad/asc0/fresh", "standard", "ironclad", 0,
        Array.Empty<string>(), Acts,
        SeededRunStandardAdmission.StandardDefaultSelectionPolicy,
        new SeededRunContextProfileBaseline("fresh", "fresh-profile", Digest),
        "enabled",
        new SeededRunContextCompatibility(
            new SeededRunContextIdentityDigest("sts2/0.1.0.0", Digest),
            new SeededRunContextIdentityDigest("ai-ascension/sts2-game-mod", Digest)),
        Digest);

    private static bool Rejected(SeededRunSelectionContext context) =>
        !SeededRunStandardAdmission.HasSupportedStaticContext(context, out string error)
        && error == "unsupported_standard_context";

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }

        Console.WriteLine("PASS: " + message);
    }
}
