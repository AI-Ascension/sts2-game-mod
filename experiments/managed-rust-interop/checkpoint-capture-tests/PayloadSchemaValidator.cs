// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Minimal JSON Schema 2020-12 evaluator covering exactly the keyword set used by
/// <c>schemas/checkpoint-payload-v1.schema.json</c>: local <c>$ref</c>, <c>type</c>,
/// <c>properties</c>, <c>required</c>, <c>additionalProperties</c>, <c>const</c>, <c>enum</c>,
/// <c>oneOf</c>, <c>allOf</c>, <c>if</c>/<c>then</c>, <c>items</c>, <c>minItems</c>,
/// <c>maxItems</c>, <c>minLength</c>, <c>maxLength</c>, <c>pattern</c>, <c>minimum</c>, and
/// <c>maximum</c>. An unsupported keyword is an error, never silently ignored. Its own
/// correctness is checked against the pinned conformance fixtures before it judges a capture.
/// </summary>
internal sealed class PayloadSchemaValidator
{
    private static readonly HashSet<string> Annotations = new(StringComparer.Ordinal)
    {
        "$schema", "$id", "title", "description", "$defs"
    };

    private readonly JsonElement _root;

    internal PayloadSchemaValidator(string schemaJson)
    {
        using var document = JsonDocument.Parse(schemaJson);
        _root = document.RootElement.Clone();
    }

    internal IReadOnlyList<string> Validate(JsonElement instance)
    {
        var errors = new List<string>();
        Evaluate(_root, instance, "#", errors);
        return errors;
    }

    private void Evaluate(JsonElement schema, JsonElement instance, string path, List<string> errors)
    {
        foreach (JsonProperty keyword in schema.EnumerateObject())
        {
            if (Annotations.Contains(keyword.Name))
                continue;
            switch (keyword.Name)
            {
                case "$ref":
                    Evaluate(Resolve(keyword.Value.GetString()!), instance, path, errors);
                    break;
                case "type":
                    if (!TypeMatches(keyword.Value.GetString()!, instance))
                        errors.Add(path + ": expected " + keyword.Value.GetString());
                    break;
                case "properties":
                    if (instance.ValueKind == JsonValueKind.Object)
                        foreach (JsonProperty property in keyword.Value.EnumerateObject())
                            if (instance.TryGetProperty(property.Name, out JsonElement child))
                                Evaluate(property.Value, child, path + "/" + property.Name, errors);
                    break;
                case "required":
                    if (instance.ValueKind == JsonValueKind.Object)
                        foreach (JsonElement name in keyword.Value.EnumerateArray())
                            if (!instance.TryGetProperty(name.GetString()!, out _))
                                errors.Add(path + ": missing " + name.GetString());
                    break;
                case "additionalProperties":
                    if (instance.ValueKind == JsonValueKind.Object && !keyword.Value.GetBoolean())
                        CheckAdditional(schema, instance, path, errors);
                    break;
                case "const":
                    if (!JsonEquals(keyword.Value, instance))
                        errors.Add(path + ": const mismatch");
                    break;
                case "enum":
                    if (!Contains(keyword.Value, instance))
                        errors.Add(path + ": not in enum");
                    break;
                case "oneOf":
                    if (CountMatches(keyword.Value, instance, path) != 1)
                        errors.Add(path + ": oneOf did not match exactly one branch");
                    break;
                case "allOf":
                    foreach (JsonElement branch in keyword.Value.EnumerateArray())
                        Evaluate(branch, instance, path, errors);
                    break;
                case "if":
                    if (Matches(keyword.Value, instance, path)
                        && schema.TryGetProperty("then", out JsonElement then))
                        Evaluate(then, instance, path, errors);
                    break;
                case "then":
                    break;
                case "items":
                    if (instance.ValueKind == JsonValueKind.Array)
                    {
                        int index = 0;
                        foreach (JsonElement item in instance.EnumerateArray())
                            Evaluate(keyword.Value, item,
                                path + "/" + index++.ToString(CultureInfo.InvariantCulture), errors);
                    }
                    break;
                case "minItems":
                    if (instance.ValueKind == JsonValueKind.Array
                        && instance.GetArrayLength() < keyword.Value.GetInt32())
                        errors.Add(path + ": too few items");
                    break;
                case "maxItems":
                    if (instance.ValueKind == JsonValueKind.Array
                        && instance.GetArrayLength() > keyword.Value.GetInt32())
                        errors.Add(path + ": too many items");
                    break;
                case "minLength":
                    if (instance.ValueKind == JsonValueKind.String
                        && instance.GetString()!.Length < keyword.Value.GetInt32())
                        errors.Add(path + ": too short");
                    break;
                case "maxLength":
                    if (instance.ValueKind == JsonValueKind.String
                        && instance.GetString()!.Length > keyword.Value.GetInt32())
                        errors.Add(path + ": too long");
                    break;
                case "pattern":
                    if (instance.ValueKind == JsonValueKind.String
                        && !Regex.IsMatch(instance.GetString()!, keyword.Value.GetString()!,
                            RegexOptions.CultureInvariant, TimeSpan.FromSeconds(1)))
                        errors.Add(path + ": pattern mismatch");
                    break;
                case "minimum":
                    if (instance.ValueKind == JsonValueKind.Number
                        && instance.GetDouble() < keyword.Value.GetDouble())
                        errors.Add(path + ": below minimum");
                    break;
                case "maximum":
                    if (instance.ValueKind == JsonValueKind.Number
                        && instance.GetDouble() > keyword.Value.GetDouble())
                        errors.Add(path + ": above maximum");
                    break;
                default:
                    throw new InvalidOperationException("unsupported schema keyword: " + keyword.Name);
            }
        }
    }

    private JsonElement Resolve(string reference)
    {
        const string prefix = "#/$defs/";
        if (!reference.StartsWith(prefix, StringComparison.Ordinal))
            throw new InvalidOperationException("unsupported $ref: " + reference);
        if (!_root.GetProperty("$defs").TryGetProperty(reference.AsSpan(prefix.Length), out JsonElement resolved))
            throw new InvalidOperationException("unsupported $ref: " + reference);
        return resolved;
    }

    private static void CheckAdditional(
        JsonElement schema, JsonElement instance, string path, List<string> errors)
    {
        JsonElement declared = schema.TryGetProperty("properties", out JsonElement properties)
            ? properties
            : default;
        foreach (JsonProperty property in instance.EnumerateObject())
            if (declared.ValueKind != JsonValueKind.Object
                || !declared.TryGetProperty(property.Name, out _))
                errors.Add(path + ": unexpected property " + property.Name);
    }

    private int CountMatches(JsonElement branches, JsonElement instance, string path)
    {
        int matches = 0;
        foreach (JsonElement branch in branches.EnumerateArray())
            if (Matches(branch, instance, path))
                matches++;
        return matches;
    }

    private bool Matches(JsonElement schema, JsonElement instance, string path)
    {
        var errors = new List<string>();
        Evaluate(schema, instance, path, errors);
        return errors.Count == 0;
    }

    private static bool TypeMatches(string type, JsonElement instance) => type switch
    {
        "object" => instance.ValueKind == JsonValueKind.Object,
        "array" => instance.ValueKind == JsonValueKind.Array,
        "string" => instance.ValueKind == JsonValueKind.String,
        "integer" => instance.ValueKind == JsonValueKind.Number
            && !instance.GetRawText().Contains('.', StringComparison.Ordinal)
            && !instance.GetRawText().Contains('e', StringComparison.OrdinalIgnoreCase),
        "number" => instance.ValueKind == JsonValueKind.Number,
        "boolean" => instance.ValueKind is JsonValueKind.True or JsonValueKind.False,
        _ => throw new InvalidOperationException("unsupported type: " + type)
    };

    private static bool Contains(JsonElement candidates, JsonElement instance)
    {
        foreach (JsonElement candidate in candidates.EnumerateArray())
            if (JsonEquals(candidate, instance))
                return true;
        return false;
    }

    private static bool JsonEquals(JsonElement left, JsonElement right)
    {
        if (left.ValueKind != right.ValueKind)
            return false;
        return left.ValueKind switch
        {
            JsonValueKind.String => string.Equals(left.GetString(), right.GetString(),
                StringComparison.Ordinal),
            JsonValueKind.Number => left.GetRawText() == right.GetRawText(),
            JsonValueKind.True or JsonValueKind.False or JsonValueKind.Null => true,
            _ => left.GetRawText() == right.GetRawText()
        };
    }
}
