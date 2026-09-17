// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string LookupBindingProfile = "game-information-lookup-binding-v1";
    private const string LookupBindingSchemaDigest =
        "f10f9af01d6be1de104069ba842e7971971e88f27553e782e81174ee7aa1cd58";

    private static (int Status, string Response) ProcessLookupBindingWork(
        RuntimeContext context,
        string body)
    {
        if (!LookupBindingRequestIsClosed(body))
        {
            return (400, LookupBindingError(context, "malformed"));
        }

        try
        {
            using JsonDocument request = JsonDocument.Parse(body);
            JsonElement root = request.RootElement;
            string locale = context.Locale;
            if (!LookupBindingManifest.ValidLocale(locale))
            {
                return (400, LookupBindingError(context, "malformed"));
            }
            if (!TryAuthorizeRuntimeV2Context(context, out _))
            {
                return (409, LookupBindingError(context, "missing_capability"));
            }
            LookupBindingManifest manifest = LookupBindingManifest.Read();
            string bindingId = LookupBindingManifest.Digest(
                JsonSerializer.Serialize(new SortedDictionary<string, object?>
                {
                    ["agent_id"] = root.GetProperty("agent_id").GetString(),
                    ["authority_epoch"] = root.GetProperty("authority_epoch").GetUInt64(),
                    ["content_manifest_id"] = manifest.ContentManifestId,
                    ["episode_id"] = root.GetProperty("episode_id").GetString(),
                    ["game_profile"] = "sts2",
                    ["locale"] = locale,
                    ["project_id"] = root.GetProperty("project_id").GetString(),
                    ["run_id"] = root.GetProperty("run_id").GetString()
                }));
            var binding = new Dictionary<string, object?>
            {
                ["binding_id"] = bindingId,
                ["scope"] = new Dictionary<string, string>
                {
                    ["project_id"] = root.GetProperty("project_id").GetString()!,
                    ["run_id"] = root.GetProperty("run_id").GetString()!,
                    ["episode_id"] = root.GetProperty("episode_id").GetString()!,
                    ["agent_id"] = root.GetProperty("agent_id").GetString()!
                },
                ["game_profile"] = "sts2",
                ["content_manifest_id"] = manifest.ContentManifestId,
                ["locale"] = locale,
                ["authority_epoch"] = root.GetProperty("authority_epoch").GetUInt64(),
                ["instance_id"] = context.InstanceId,
                ["authority"] = new Dictionary<string, string>
                {
                    ["scope"] = "sts2-harness",
                    ["instance_lease"] = "sts2-gateway",
                    ["content_revision"] = "sts2-game-mod"
                }
            };
            LiveCardCapturedSnapshot liveSnapshot = ReadLiveCardSnapshot(
                context.InstanceId, manifest.ContentManifestId);
            if (liveSnapshot.Available)
            {
                AssociateLiveCardBinding(context, root, liveSnapshot);
            }
            if (root.GetProperty("operation").GetString() == "discovery")
            {
                return (200, LookupBindingResponse(context, "lookup_binding_discovery_response",
                    binding, new Dictionary<string, object?>
                    {
                        ["observation_state"] = "not_yet_observed",
                        ["required_capabilities"] = new Dictionary<string, string>
                        {
                            ["profile"] = LookupBindingProfile,
                            ["schema_digest"] = LookupBindingSchemaDigest
                        },
                        ["reobserve"] = null
                    }, null));
            }
            if (!LiveCombatSource.TryReadCurrentGeneration(out ulong generation))
            {
                return (503, LookupBindingError(context, "missing_capability"));
            }
            string snapshotId = $"live:{generation}";
            return (200, LookupBindingResponse(context, "lookup_binding_observation_response",
                binding, new Dictionary<string, object?>
                {
                    ["observation_state"] = "observed",
                    ["required_capabilities"] = new Dictionary<string, string>
                    {
                        ["profile"] = LookupBindingProfile,
                        ["schema_digest"] = LookupBindingSchemaDigest
                    },
                    ["reobserve"] = null
                }, new Dictionary<string, object?>
                {
                    ["observation_id"] = $"observation:{generation}",
                    ["binding_id"] = bindingId,
                    ["snapshot_id"] = snapshotId,
                    ["state_generation"] = generation
                }));
        }
        catch (Exception)
        {
            return (503, LookupBindingError(context, "missing_capability"));
        }
    }

    private static bool LookupBindingRequestIsClosed(string body)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(body, new JsonDocumentOptions
            {
                MaxDepth = 8
            });
            JsonElement root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
            {
                return false;
            }
            var expected = new HashSet<string>
            {
                "operation", "project_id", "run_id", "episode_id", "agent_id", "authority_epoch"
            };
            foreach (JsonProperty property in root.EnumerateObject())
            {
                if (!expected.Remove(property.Name))
                {
                    return false;
                }
            }
            return expected.Count == 0
                && root.GetProperty("operation").ValueKind == JsonValueKind.String
                && (root.GetProperty("operation").GetString() is "discovery" or "observe")
                && LookupBindingIdentity(root.GetProperty("project_id"))
                && LookupBindingIdentity(root.GetProperty("run_id"))
                && LookupBindingIdentity(root.GetProperty("episode_id"))
                && LookupBindingIdentity(root.GetProperty("agent_id"))
                && root.GetProperty("authority_epoch").TryGetUInt64(out _);
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool LookupBindingIdentity(JsonElement value) =>
        value.ValueKind == JsonValueKind.String
        && RuntimeV3GameplayContract.IsIdentity(value.GetString() ?? string.Empty);

    private static string LookupBindingError(RuntimeContext context, string code) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = LookupBindingProfile,
            ["schema_digest"] = LookupBindingSchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = "sts2-protocol/game-information-lookup-binding-v1",
                ["source"] = "schemas/game-information-lookup-binding-v1.schema.json",
                ["generator"] = "hand-authored"
            },
            ["correlation_id"] = context.CorrelationId,
            ["kind"] = "error_response",
            ["binding"] = null,
            ["discovery"] = null,
            ["observation"] = null,
            ["error"] = new Dictionary<string, object?>
            {
                ["code"] = code,
                ["field"] = null,
                ["reason"] = null
            }
        });

    private static string LookupBindingResponse(
        RuntimeContext context,
        string kind,
        Dictionary<string, object?> binding,
        Dictionary<string, object?> discovery,
        Dictionary<string, object?>? observation) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = LookupBindingProfile,
            ["schema_digest"] = LookupBindingSchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = "sts2-protocol/game-information-lookup-binding-v1",
                ["source"] = "schemas/game-information-lookup-binding-v1.schema.json",
                ["generator"] = "hand-authored"
            },
            ["correlation_id"] = context.CorrelationId,
            ["kind"] = kind,
            ["binding"] = binding,
            ["discovery"] = discovery,
            ["observation"] = observation,
            ["error"] = null
        });
}

/// <summary>Reads the installed host's model registries on the queued game thread.</summary>
internal sealed class LookupBindingManifest
{
    private LookupBindingManifest(string contentManifestId)
    {
        ContentManifestId = contentManifestId;
    }

    internal string ContentManifestId { get; }

    internal static LookupBindingManifest Read()
    {
        // ModelDb exposes IDs, but #83's canonical ContentManifestProducer also requires
        // build, package order/version, semantic inputs, locale text, provenance, override
        // chains, and a before/after generation witness. When that source is available, the
        // binding must use its exact inventory_revision. Do not publish an ID-only digest or a
        // separately reconstructed hash as content_manifest_id.
        throw new InvalidOperationException("typed content manifest source unavailable");
    }

    internal static bool ValidLocale(string value) =>
        value.Length is >= 2 and <= 35
        && value.Split('-').All(part => part.Length is >= 2 and <= 8
            && part.All(char.IsLetterOrDigit));

    internal static string Digest(string value) =>
        Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value))).ToLowerInvariant();

}
