// SPDX-License-Identifier: MIT

using System;
using System.Buffers;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Managed mirror of the selected-context portion of sts2-protocol/seeded-run-v1.
/// The names and canonical serialization order intentionally match the Rust contract.
/// </summary>
internal sealed record SeededRunContextProfileBaseline(
    [property: JsonPropertyName("kind")] string Kind,
    [property: JsonPropertyName("identity")] string Identity,
    [property: JsonPropertyName("digest")] string Digest);

internal sealed record SeededRunContextIdentityDigest(
    [property: JsonPropertyName("identity")] string Identity,
    [property: JsonPropertyName("digest")] string Digest);

internal sealed record SeededRunContextCompatibility(
    [property: JsonPropertyName("game")] SeededRunContextIdentityDigest Game,
    [property: JsonPropertyName("mod")] SeededRunContextIdentityDigest Mod);

internal sealed record SeededRunSelectionContext(
    [property: JsonPropertyName("context_id")] string ContextId,
    [property: JsonPropertyName("game_mode")] string GameMode,
    [property: JsonPropertyName("character")] string Character,
    [property: JsonPropertyName("ascension")] int Ascension,
    [property: JsonPropertyName("modifiers")] IReadOnlyList<string> Modifiers,
    [property: JsonPropertyName("acts")] IReadOnlyList<string> Acts,
    [property: JsonPropertyName("selection_policy")] string SelectionPolicy,
    [property: JsonPropertyName("profile_baseline")] SeededRunContextProfileBaseline ProfileBaseline,
    [property: JsonPropertyName("save_policy")] string SavePolicy,
    [property: JsonPropertyName("compatibility")] SeededRunContextCompatibility Compatibility,
    [property: JsonPropertyName("context_digest")] string ContextDigest)
{
    internal const string StandardGameMode = "standard";
    internal const string IroncladCharacter = "ironclad";
    internal const string FreshProfileKind = "fresh";
    internal const string EnabledSavePolicy = "enabled";
    internal const string StandardDefaultSelectionPolicy = "standard_default";

    internal bool Validate(out string error)
    {
        if (!SeededRunStandardContract.IsContextIdentity(ContextId)
            || GameMode != StandardGameMode
            || Character != IroncladCharacter
            || Ascension is < 0 or > 20
            || Modifiers is null || Acts is null
            || Modifiers.Count > SeededRunStandardContract.MaxModifiers
            || Acts.Count is < 1 or > SeededRunStandardContract.MaxActs
            || !SeededRunStandardContract.IsContextText(SelectionPolicy)
            || ProfileBaseline is null
            || Compatibility is null
            || SavePolicy is not ("disabled" or "enabled")
            || !SeededRunStandardContract.IsDigest(ContextDigest))
        {
            error = "selected context has invalid bounds or native standard values";
            return false;
        }

        if (Modifiers.Any(value => !SeededRunStandardContract.IsContextText(value))
            || Modifiers.Zip(Modifiers.Skip(1), (left, right) =>
                string.CompareOrdinal(left, right) >= 0).Any(result => result)
            || Acts.Any(value => !SeededRunStandardContract.IsContextText(value))
            || Acts.Distinct(StringComparer.Ordinal).Count() != Acts.Count)
        {
            error = "selected context lists are invalid or not in native order";
            return false;
        }

        if (!SeededRunStandardContract.ValidateProfileBaseline(ProfileBaseline, out error)
            || !SeededRunStandardContract.ValidateCompatibility(Compatibility, out error))
        {
            return false;
        }

        string expected = CanonicalDigest();
        if (!string.Equals(expected, ContextDigest, StringComparison.Ordinal))
        {
            error = "selected context digest does not match its canonical fields";
            return false;
        }

        error = string.Empty;
        return true;
    }

    /// <summary>Returns the Rust-compatible SHA-256 over fields excluding context_digest.</summary>
    internal string CanonicalDigest() =>
        Convert.ToHexString(SHA256.HashData(CanonicalJsonBytes())).ToLowerInvariant();

    internal string CanonicalJson() => Encoding.UTF8.GetString(CanonicalJsonBytes());

    private byte[] CanonicalJsonBytes()
    {
        var output = new ArrayBufferWriter<byte>();
        using (var writer = new Utf8JsonWriter(output, new JsonWriterOptions { Indented = false }))
        {
            // Keep this order synchronized with SeededRunContextDigestInput in the Rust crate.
            writer.WriteStartObject();
            writer.WriteString("context_id", ContextId);
            writer.WriteString("game_mode", GameMode);
            writer.WriteString("character", Character);
            writer.WriteNumber("ascension", Ascension);
            WriteArray(writer, "modifiers", Modifiers);
            WriteArray(writer, "acts", Acts);
            writer.WriteString("selection_policy", SelectionPolicy);
            writer.WritePropertyName("profile_baseline");
            writer.WriteStartObject();
            writer.WriteString("kind", ProfileBaseline.Kind);
            writer.WriteString("identity", ProfileBaseline.Identity);
            writer.WriteString("digest", ProfileBaseline.Digest);
            writer.WriteEndObject();
            writer.WriteString("save_policy", SavePolicy);
            writer.WritePropertyName("compatibility");
            writer.WriteStartObject();
            WriteIdentityDigest(writer, "game", Compatibility.Game);
            WriteIdentityDigest(writer, "mod", Compatibility.Mod);
            writer.WriteEndObject();
            writer.WriteEndObject();
            writer.Flush();
        }

        return output.WrittenSpan.ToArray();
    }

    private static void WriteArray(Utf8JsonWriter writer, string name, IReadOnlyList<string> values)
    {
        writer.WritePropertyName(name);
        writer.WriteStartArray();
        foreach (string value in values)
        {
            writer.WriteStringValue(value);
        }

        writer.WriteEndArray();
    }

    private static void WriteIdentityDigest(
        Utf8JsonWriter writer,
        string name,
        SeededRunContextIdentityDigest value)
    {
        writer.WritePropertyName(name);
        writer.WriteStartObject();
        writer.WriteString("identity", value.Identity);
        writer.WriteString("digest", value.Digest);
        writer.WriteEndObject();
    }
}

internal sealed record SeededRunStandardRequest(
    [property: JsonPropertyName("operation_id")] string OperationId,
    [property: JsonPropertyName("requested_seed")] string RequestedSeed,
    [property: JsonPropertyName("run_mode")] string RunMode,
    [property: JsonPropertyName("context_digest")] string ContextDigest,
    [property: JsonPropertyName("selected_context")] SeededRunSelectionContext SelectedContext)
{
    internal bool Validate(out string error)
    {
        if (!SeededRunStandardContract.IsIdentity(OperationId)
            || !SeededRunStandardContract.IsSeed(RequestedSeed)
            || RunMode is not ("seeded_training" or "seeded_replay" or "diagnostic")
            || SelectedContext is null
            || !SeededRunStandardContract.IsDigest(ContextDigest))
        {
            error = "seeded-run request is outside the bounded host contract";
            return false;
        }

        if (!SelectedContext.Validate(out error))
        {
            return false;
        }

        if (!string.Equals(ContextDigest, SelectedContext.ContextDigest, StringComparison.Ordinal))
        {
            error = "top-level context digest does not match selected_context";
            return false;
        }

        error = string.Empty;
        return true;
    }

    internal string Fingerprint() =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(
            $"{OperationId}\n{RequestedSeed}\n{RunMode}\n{ContextDigest}\n{SelectedContext.CanonicalJson()}")))
            .ToLowerInvariant();
}

internal static class SeededRunStandardContract
{
    internal const int MaxIdentityBytes = 128;
    internal const int MaxContextTextBytes = 128;
    internal const int MaxSeedBytes = 64;
    internal const int MaxModifiers = 32;
    internal const int MaxActs = 8;

    internal static bool IsIdentity(string? value) =>
        !string.IsNullOrEmpty(value)
        && Encoding.UTF8.GetByteCount(value) <= MaxIdentityBytes
        && value.All(character => character <= 0x7f
            && (char.IsAsciiLetterOrDigit(character) || ".:/-_".Contains(character)));

    internal static bool IsContextIdentity(string? value) => IsIdentity(value);

    internal static bool IsContextText(string? value) =>
        !string.IsNullOrEmpty(value)
        && Encoding.UTF8.GetByteCount(value) <= MaxContextTextBytes
        && value.All(character => character <= 0x7f
            && (char.IsAsciiLetterOrDigit(character) || ".:/-_".Contains(character)));

    internal static bool IsSeed(string? value) =>
        !string.IsNullOrEmpty(value)
        && Encoding.UTF8.GetByteCount(value) <= MaxSeedBytes
        && value.All(character => !char.IsControl(character));

    internal static bool IsDigest(string? value) =>
        value?.Length == 64
        && value.All(character => char.IsAsciiDigit(character)
            || character is >= 'a' and <= 'f');

    internal static bool ValidateProfileBaseline(
        SeededRunContextProfileBaseline baseline,
        out string error)
    {
        if (baseline.Kind is not ("fresh" or "existing")
            || !IsIdentity(baseline.Identity)
            || !IsDigest(baseline.Digest))
        {
            error = "profile baseline identity or digest is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }

    private static bool ValidateIdentityDigest(
        SeededRunContextIdentityDigest value,
        out string error)
    {
        if (!IsIdentity(value.Identity) || !IsDigest(value.Digest))
        {
            error = "compatibility identity or digest is invalid";
            return false;
        }

        error = string.Empty;
        return true;
    }

    internal static bool ValidateCompatibility(
        SeededRunContextCompatibility compatibility,
        out string error)
    {
        if (compatibility.Game is null || compatibility.Mod is null)
        {
            error = "compatibility identities are missing";
            return false;
        }

        return ValidateIdentityDigest(compatibility.Game, out error)
            && ValidateIdentityDigest(compatibility.Mod, out error);
    }
}
