// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// One entry inside a host-offered set, carrying only what the host actually supplied.
///
/// <para>
/// This is the producer half of <c>sts2-game-mod#171</c>. A model told only
/// <c>["card:21:Setup-Strike", ...]</c> cannot rank the options, so it skips, and a skipped reward
/// is offered again. Emitting the host's own display attributes is what makes the choice non-blind.
/// </para>
///
/// <para>
/// <b>Every attribute here is the host's, never this loader's.</b> An attribute the host did not
/// supply stays <see langword="null"/> and is omitted from the JSON; it is never defaulted, inferred,
/// or filled from a catalog. Nothing in this file emits a rarity, a probability, a generation
/// weight, a reroll rule, or any undisclosed outcome: there is no such table in the shipped
/// composition, and adding one would turn an absence into an invented fact.
/// </para>
///
/// <para>
/// <b>Recorded negative: no host surface populates <see cref="Contents"/>.</b> The harness admits
/// <c>contents</c> on any Choice and the type below carries it, but <c>NRewardsScreen</c> exposes
/// only <c>RewardsSetIndex</c> and the reward type name through <c>NRewardButton.Reward</c> — the
/// cards behind a reward live behind the *next* overlay, whose model is not reachable from this
/// loader without the source-only content catalog. So <c>contents</c> is supported, serialized, and
/// validated, and no producer fills it. That is stated here rather than left implicit, because an
/// unfilled field is otherwise indistinguishable from a missing one.
/// </para>
/// </summary>
internal sealed record RuntimeV3GameplayOfferedEntry(
    string ChoiceId,
    string? Name = null,
    int? Cost = null,
    bool? Upgraded = null,
    string? Description = null,
    string? Rarity = null,
    IReadOnlyList<RuntimeV3GameplayOfferedContent>? Contents = null)
{
    /// <summary>
    /// The bare-identifier entry, which is the exact form every host emitted before the observation
    /// was widened. The codec still renders it as a JSON string so undescribed hosts are unaffected.
    /// </summary>
    internal static RuntimeV3GameplayOfferedEntry Identified(string choiceId) =>
        new(choiceId);

    /// <summary>
    /// Builds an entry from a host card, publishing each attribute only when the host read it.
    ///
    /// <para>
    /// The withheld-attribute rule is explicit here rather than left to the codec: a host that
    /// cannot read a field contributes nothing for it. <paramref name="description"/> is accepted
    /// because a host card may carry its own rules text, but this loader never synthesizes one.
    /// </para>
    /// </summary>
    internal static RuntimeV3GameplayOfferedEntry FromCard(string choiceId, string? title,
        int? energyCost, bool? upgraded, string? description = null, string? rarity = null,
        IReadOnlyList<RuntimeV3GameplayOfferedContent>? contents = null)
    {
        return new RuntimeV3GameplayOfferedEntry(
            choiceId,
            RuntimeV3GameplayContract.IsPublishableText(title) ? title : null,
            energyCost is { } cost && RuntimeV3GameplayContract.IsPublishableCost(cost) ? cost : null,
            upgraded,
            RuntimeV3GameplayContract.IsPublishableText(description) ? description : null,
            RuntimeV3GameplayContract.IsPublishableText(rarity) ? rarity : null,
            contents is { Count: > 0 } ? contents : null);
    }

    /// <summary>Whether this entry carries nothing beyond its identity.</summary>
    internal bool IsIdentifiedOnly =>
        Name is null && Cost is null && !Upgraded.HasValue
        && Description is null && Rarity is null
        && Contents is null or { Count: 0 };

    /// <summary>
    /// Rejects an entry whose identity, supplied text, supplied cost, or contents break a bound the
    /// harness enforces. Validation runs before serialization so a producer bug surfaces as a
    /// refused observation rather than as JSON the harness will later reject.
    /// </summary>
    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(ChoiceId))
        {
            error = "offered entry identity is invalid";
            return false;
        }
        // Withheld text arrives as null, so each attribute must be either absent or publishable:
        // a non-null value that breaks the 512-byte bound is a producer bug, not an absence.
        if (Name is not null && !RuntimeV3GameplayContract.IsPublishableText(Name)
            || Description is not null && !RuntimeV3GameplayContract.IsPublishableText(Description)
            || Rarity is not null && !RuntimeV3GameplayContract.IsPublishableText(Rarity))
        {
            error = "offered entry text breaks the observation bound";
            return false;
        }
        if (Cost is { } cost && !RuntimeV3GameplayContract.IsPublishableCost(cost))
        {
            error = "offered entry cost is outside the bound the harness accepts";
            return false;
        }
        if (Contents is { Count: > 0 })
        {
            if (Contents.Count > RuntimeV3GameplayContract.MaxChoiceContents)
            {
                error = "offered entry contents exceed its bound";
                return false;
            }
            foreach (RuntimeV3GameplayOfferedContent content in Contents)
            {
                if (!content.Validate(out error))
                {
                    return false;
                }
            }
        }
        error = string.Empty;
        return true;
    }
}

/// <summary>
/// One entry inside another entry's <c>contents</c>: what taking this option would present next.
///
/// <para>
/// This is a distinct type rather than the same entry type, which is what keeps disclosure one level
/// deep structurally. A reward is chosen on one screen and its cards appear on the next, so
/// <c>contents</c> is what closes the blind first choice; but a content entry has no
/// <c>contents</c> of its own, mirroring the harness's own <c>CHOICE_CONTENT_FIELDS</c> split.
/// </para>
/// </summary>
internal sealed record RuntimeV3GameplayOfferedContent(
    string ChoiceId,
    string? Name = null,
    int? Cost = null,
    bool? Upgraded = null,
    string? Description = null,
    string? Rarity = null)
{
    /// <summary>The bare-identifier content, for a host that supplied no further detail.</summary>
    internal static RuntimeV3GameplayOfferedContent Identified(string choiceId) =>
        new(choiceId);

    /// <summary>
    /// Builds a content entry from a host card, publishing each attribute only when the host read
    /// it. Identical withholding rules to the top-level entry, for the same reasons.
    /// </summary>
    internal static RuntimeV3GameplayOfferedContent FromCard(string choiceId, string? title,
        int? energyCost, bool? upgraded, string? description = null, string? rarity = null) =>
        new(choiceId,
            RuntimeV3GameplayContract.IsPublishableText(title) ? title : null,
            energyCost is { } cost && RuntimeV3GameplayContract.IsPublishableCost(cost) ? cost : null,
            upgraded,
            RuntimeV3GameplayContract.IsPublishableText(description) ? description : null,
            RuntimeV3GameplayContract.IsPublishableText(rarity) ? rarity : null);

    /// <summary>Whether this entry carries nothing beyond its identity.</summary>
    internal bool IsIdentifiedOnly =>
        Name is null && Cost is null && !Upgraded.HasValue
        && Description is null && Rarity is null;

    /// <summary>Rejects a content entry whose identity, text, or cost breaks a bound.</summary>
    internal bool Validate(out string error)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(ChoiceId))
        {
            error = "offered content identity is invalid";
            return false;
        }
        if (Name is not null && !RuntimeV3GameplayContract.IsPublishableText(Name)
            || Description is not null && !RuntimeV3GameplayContract.IsPublishableText(Description)
            || Rarity is not null && !RuntimeV3GameplayContract.IsPublishableText(Rarity))
        {
            error = "offered content text breaks the observation bound";
            return false;
        }
        if (Cost is { } cost && !RuntimeV3GameplayContract.IsPublishableCost(cost))
        {
            error = "offered content cost is outside the bound the harness accepts";
            return false;
        }
        error = string.Empty;
        return true;
    }
}

/// <summary>A host-offered set, in the order the host listed it.</summary>
internal sealed record RuntimeV3GameplayOfferedSet(
    IReadOnlyList<RuntimeV3GameplayOfferedEntry> Entries)
{
    internal static readonly RuntimeV3GameplayOfferedSet Empty =
        new(Array.Empty<RuntimeV3GameplayOfferedEntry>());

    /// <summary>
    /// Collects a set from bare identifiers, the form a host uses when it has no further detail.
    /// </summary>
    internal static RuntimeV3GameplayOfferedSet Identified(IReadOnlyList<string> identities)
    {
        var entries = new List<RuntimeV3GameplayOfferedEntry>(identities.Count);
        foreach (string identity in identities)
        {
            entries.Add(RuntimeV3GameplayOfferedEntry.Identified(identity));
        }
        return new RuntimeV3GameplayOfferedSet(entries);
    }

    /// <summary>
    /// Validates the set: bounds, per-entry shape, and entry-identity uniqueness.
    ///
    /// <para>
    /// Duplicates are refused because the catalog cannot act on them: a <c>choose_reward</c> action
    /// carries exactly one <c>reward_id</c>, so two entries sharing an identity make the selection
    /// ambiguous. Refusing the observation is better than emitting one a consumer cannot resolve.
    /// The check is per set: one identity appearing as a top-level entry and again inside that
    /// entry's <c>contents</c> is two sets on two different screens and is allowed.
    /// </para>
    /// </summary>
    internal bool Validate(out string error)
    {
        if (Entries.Count > RuntimeV3GameplayContract.MaxOfferedEntries)
        {
            error = "offered set exceeds its bound";
            return false;
        }
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (RuntimeV3GameplayOfferedEntry entry in Entries)
        {
            if (!entry.Validate(out error))
            {
                return false;
            }
            if (!seen.Add(entry.ChoiceId))
            {
                error = "offered entry identities must be unique";
                return false;
            }
        }
        error = string.Empty;
        return true;
    }
}
