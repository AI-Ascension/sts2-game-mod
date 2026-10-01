// SPDX-License-Identifier: MIT

using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// The offered-set half of the observation codec: what the producer is allowed to disclose, and in
/// which JSON shape.
///
/// <para>
/// It is split out of <see cref="RuntimeV3GameplayCodec"/> because this is the file a reviewer reads
/// when asking whether a described entry could ever carry something the host did not supply. The
/// answer is structural: a key appears only under a non-null check on a value the producer received
/// from the host, and there is no branch that supplies one.
/// </para>
/// </summary>
internal static class RuntimeV3GameplayOfferedCodec
{
    /// <summary>
    /// Renders a host-offered set as the harness accepts it: a bare identifier string per entry
    /// when the host disclosed nothing beyond the identity, or a described object when it did.
    ///
    /// <para>
    /// The bare form is load-bearing rather than cosmetic. Every host shipped before the
    /// observation was widened emits <c>["card:21:Setup-Strike", ...]</c>, so a described form
    /// emitted for those entries would change the bytes every existing consumer sees. An entry
    /// with attributes becomes the object form because that is the only way to carry them.
    /// </para>
    ///
    /// <para>
    /// Only attributes the host supplied appear. A host that could not read a field contributed
    /// nothing for it, and this codec never fills the gap: a defaulted rarity or a generated
    /// description would read downstream as an observed fact about the game.
    /// </para>
    /// </summary>
    internal static List<object> OfferSet(RuntimeV3GameplayOfferedSet offered)
    {
        var values = new List<object>(offered.Entries.Count);
        foreach (RuntimeV3GameplayOfferedEntry entry in offered.Entries)
        {
            if (entry.IsIdentifiedOnly)
            {
                values.Add(entry.ChoiceId);
                continue;
            }
            var value = new Dictionary<string, object?> { ["choice_id"] = entry.ChoiceId };
            // Each attribute is written only when the host actually supplied it.
            if (entry.Name is not null)
            {
                value["name"] = entry.Name;
            }
            if (entry.Cost is { } cost)
            {
                value["cost"] = cost;
            }
            if (entry.Upgraded is { } upgraded)
            {
                value["upgraded"] = upgraded;
            }
            if (entry.Description is not null)
            {
                value["description"] = entry.Description;
            }
            if (entry.Rarity is not null)
            {
                value["rarity"] = entry.Rarity;
            }
            if (entry.Contents is { Count: > 0 })
            {
                value["contents"] = OfferContents(entry.Contents);
            }
            values.Add(value);
        }
        return values;
    }

    /// <summary>
    /// Renders what taking an option would present next.
    ///
    /// <para>
    /// A content entry uses the same bare-or-object duality as a top-level entry, and has no
    /// <c>contents</c> key of its own: disclosure is one level deep, matching the harness's
    /// <c>CHOICE_CONTENT_FIELDS</c>, which does not list one.
    /// </para>
    /// </summary>
    private static List<object> OfferContents(
        IReadOnlyList<RuntimeV3GameplayOfferedContent> contents)
    {
        var values = new List<object>(contents.Count);
        foreach (RuntimeV3GameplayOfferedContent content in contents)
        {
            if (content.IsIdentifiedOnly)
            {
                values.Add(content.ChoiceId);
                continue;
            }
            var value = new Dictionary<string, object?> { ["choice_id"] = content.ChoiceId };
            if (content.Name is not null)
            {
                value["name"] = content.Name;
            }
            if (content.Cost is { } cost)
            {
                value["cost"] = cost;
            }
            if (content.Upgraded is { } upgraded)
            {
                value["upgraded"] = upgraded;
            }
            if (content.Description is not null)
            {
                value["description"] = content.Description;
            }
            if (content.Rarity is not null)
            {
                value["rarity"] = content.Rarity;
            }
            values.Add(value);
        }
        return values;
    }
}
