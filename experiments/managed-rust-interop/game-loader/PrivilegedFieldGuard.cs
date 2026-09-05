// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Structural guard used before any managed projection is serialized.</summary>
internal static class PrivilegedFieldGuard
{
    private static readonly string[] ForbiddenNames =
    {
        "rng", "random_state", "future", "unrevealed", "secret", "credential", "password",
        "access_token", "raw_memory", "host_object", "executable", "pck", "dll", "save_file",
        "process_command", "reflection", "screen_coordinate", "input_event", "private_prompt"
    };

    internal static bool IsSafeJson(JsonElement value, out string error)
    {
        if (value.ValueKind == JsonValueKind.Object)
        {
            foreach (JsonProperty property in value.EnumerateObject())
            {
                if (IsForbidden(property.Name))
                {
                    error = $"privileged field rejected: {property.Name}";
                    return false;
                }
                if (!IsSafeJson(property.Value, out error))
                {
                    return false;
                }
            }
        }
        else if (value.ValueKind == JsonValueKind.Array)
        {
            foreach (JsonElement child in value.EnumerateArray())
            {
                if (!IsSafeJson(child, out error))
                {
                    return false;
                }
            }
        }

        error = string.Empty;
        return true;
    }

    internal static bool IsForbidden(string name)
    {
        string normalized = name.ToLowerInvariant();
        foreach (string forbidden in ForbiddenNames)
        {
            if (normalized.Equals(forbidden, StringComparison.Ordinal)
                || normalized.Contains(forbidden, StringComparison.Ordinal))
            {
                return true;
            }
        }
        return false;
    }
}
