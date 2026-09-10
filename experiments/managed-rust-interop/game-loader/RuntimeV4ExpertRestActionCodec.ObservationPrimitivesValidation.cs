// SPDX-License-Identifier: MIT

using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool IdentityField(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && value.ValueKind == JsonValueKind.String
        && RuntimeV4ExpertRestActionContract.IsIdentity(value.GetString());

    private static bool NullableIdentity(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && (value.ValueKind == JsonValueKind.Null || IdentityField(parent, field));

    private static bool TextField(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && value.ValueKind == JsonValueKind.String
        && RuntimeV3GameplayContract.IsText(value.GetString()!);

    private static bool NullableText(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && (value.ValueKind == JsonValueKind.Null || TextField(parent, field));

    private static bool BoolField(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && value.ValueKind is JsonValueKind.True or JsonValueKind.False;

    private static bool NullableBool(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && (value.ValueKind == JsonValueKind.Null || BoolField(parent, field));

    private static bool ByteField(JsonElement parent, string field, int minimum = 0) =>
        parent.TryGetProperty(field, out JsonElement value)
        && value.ValueKind == JsonValueKind.Number
        && value.TryGetInt32(out int number)
        && number is >= 0 and <= byte.MaxValue
        && number >= minimum;

    private static bool NullableByte(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && (value.ValueKind == JsonValueKind.Null || ByteField(parent, field));

    private static bool UInt16Field(JsonElement parent, string field) =>
        UInt16Field(parent, field, out _);

    private static bool UInt16Field(JsonElement parent, string field, out ushort number)
    {
        number = 0;
        return parent.TryGetProperty(field, out JsonElement value)
            && value.ValueKind == JsonValueKind.Number
            && value.TryGetUInt16(out number);
    }

    private static bool NullableUInt16(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && (value.ValueKind == JsonValueKind.Null || UInt16Field(parent, field));

    private static bool UInt32Field(JsonElement parent, string field) =>
        parent.TryGetProperty(field, out JsonElement value)
        && value.ValueKind == JsonValueKind.Number
        && value.TryGetUInt32(out _);

    private static bool Int32Field(JsonElement parent, string field, int minimum, int maximum,
        out int number)
    {
        number = 0;
        return parent.TryGetProperty(field, out JsonElement value)
            && value.ValueKind == JsonValueKind.Number
            && value.TryGetInt32(out number)
            && number >= minimum && number <= maximum;
    }

    private static bool NullableInt(JsonElement parent, string field, int minimum, int maximum)
    {
        return parent.TryGetProperty(field, out JsonElement value)
            && (value.ValueKind == JsonValueKind.Null
                || Int32Field(parent, field, minimum, maximum, out _));
    }
}
