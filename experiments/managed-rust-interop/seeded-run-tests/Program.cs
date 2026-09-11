// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class Program
{
    private static readonly string[] InvalidModifiers = { "z", "a" };
    private static readonly string[] ReorderedActs = { "act_2", "act_1" };
    private static readonly string[] NativeActs = { "act_1", "act_2", "act_3", "act_4" };
    private static readonly string[] VerifiedNativeActs = { "OVERGROWTH", "HIVE", "GLORY" };

    private const string BaselineDigest =
        "4581aaf95348126550cdf3b73ec46b39d447523cf7cb35aec71c2842d1945031";
    private const string GameDigest =
        "2db9d9f665c776c2324c7f98134b900a8b3332f32148ea1cb84063d52db94ff4";
    private const string ModDigest =
        "0c6e7bbb54996222a4894de999fc860decc360f9936c9bd5a7382d97b361bb7e";

    private static void Main()
    {
        SeededRunSelectionContext context = Context();
        Check(context.CanonicalDigest()
            == "d57563180f198b73970510427981504a9df10c62931577e601f9dcce6275fbe9",
            "canonical digest matches Rust field order and compact JSON");
        Check(context.CanonicalJson().Contains(
                "\"selection_policy\":\"standard_default\",\"profile_baseline\"",
                StringComparison.Ordinal),
            "canonical JSON keeps selection policy before profile baseline");
        Check(context.Validate(out _), "valid selected context is admitted");

        SeededRunStandardRequest request = new(
            "operation-1", "o0i1", "seeded_training", context.ContextDigest, context);
        Check(request.Validate(out _), "top-level context digest binds selected context");
        SeededRunSelectionContext nativeContext = NativeContext();
        Check(nativeContext.Validate(out _),
            "native standard context accepts uppercase ordered act identities and enabled saves");
        Check(nativeContext.Acts.SequenceEqual(
                VerifiedNativeActs, StringComparer.Ordinal),
            "native standard context preserves the verified act order");
        Check(!(request with { ContextDigest = new string('0', 64) }).Validate(out _),
            "top-level digest mismatch is rejected");
        Check(!(context with { Modifiers = InvalidModifiers }).Validate(out _),
            "modifier order is rejected when it is not canonical");
        SeededRunSelectionContext reorderedContext = context with { Acts = ReorderedActs };
        reorderedContext = reorderedContext with { ContextDigest = reorderedContext.CanonicalDigest() };
        Check(reorderedContext.Validate(out _),
            "native act order is preserved rather than sorted");
        Check(!(context with
        {
            ContextId = "standard/ironclad/asc0/fresh",
            ContextDigest = context.ContextDigest.Replace(
                context.ContextDigest[0], context.ContextDigest[0] == '0' ? '1' : '0')
        }).Validate(out _), "changing a context field invalidates its digest");
        Console.WriteLine("seeded-run context checks passed");
    }

    private static SeededRunSelectionContext Context() => new(
        "standard/ironclad/asc0/fresh",
        "standard",
        "ironclad",
        0,
        Array.Empty<string>(),
        NativeActs,
        "standard_default",
        new SeededRunContextProfileBaseline("fresh", "fresh-standard-comparison", BaselineDigest),
        "disabled",
        new SeededRunContextCompatibility(
            new SeededRunContextIdentityDigest("sts2-game/v0.107.1", GameDigest),
            new SeededRunContextIdentityDigest("ai-ascension/sts2-game-mod", ModDigest)),
        "d57563180f198b73970510427981504a9df10c62931577e601f9dcce6275fbe9");

    private static SeededRunSelectionContext NativeContext()
    {
        SeededRunSelectionContext context = new(
            "standard/ironclad/asc0/fresh",
            "standard",
            "ironclad",
            0,
            Array.Empty<string>(),
            VerifiedNativeActs,
            "standard_default",
            new SeededRunContextProfileBaseline("fresh", "fresh-standard-comparison", BaselineDigest),
            "enabled",
            new SeededRunContextCompatibility(
                new SeededRunContextIdentityDigest("sts2/0.1.0.0", GameDigest),
                new SeededRunContextIdentityDigest("ai-ascension/sts2-game-mod", ModDigest)),
            string.Empty);
        return context with { ContextDigest = context.CanonicalDigest() };
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
        {
            throw new InvalidOperationException(message);
        }

        Console.WriteLine("PASS: " + message);
    }
}
