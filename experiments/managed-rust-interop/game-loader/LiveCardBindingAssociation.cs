// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static LiveCardBindingAssociation? _liveCardBinding;

    private static void AssociateLiveCardBinding(
        RuntimeContext context,
        JsonElement scope,
        LiveCardCapturedSnapshot snapshot)
    {
        _liveCardBinding = new LiveCardBindingAssociation(
            context.InstanceId,
            context.CallerId,
            context.SessionId,
            context.LeaseId,
            ParseEpoch(context.LeaseEpoch),
            StringField(scope, "project_id") ?? string.Empty,
            StringField(scope, "run_id") ?? string.Empty,
            StringField(scope, "episode_id") ?? string.Empty,
            StringField(scope, "agent_id") ?? string.Empty,
            scope.GetProperty("authority_epoch").GetUInt64(),
            snapshot.ContentManifest,
            snapshot.RunId,
            snapshot.SourceIncarnation,
            snapshot.Epoch,
            snapshot.StateGeneration);
    }

    private static bool TryReadLiveCardBinding(
        RuntimeContext context,
        string runId,
        string contentManifestId,
        ulong authorityEpoch,
        LiveCardCapturedSnapshot snapshot)
    {
        LiveCardBindingAssociation? binding = _liveCardBinding;
        return binding is not null
            && binding.InstanceId == context.InstanceId
            && binding.CallerId == context.CallerId
            && binding.SessionId == context.SessionId
            && binding.LeaseId == context.LeaseId
            && binding.LeaseEpoch == ParseEpoch(context.LeaseEpoch)
            && binding.RunId == runId
            && binding.AuthorityEpoch == authorityEpoch
            && binding.ContentManifestId == contentManifestId
            && binding.NativeRunId == snapshot.RunId
            && binding.SourceIncarnation == snapshot.SourceIncarnation
            && binding.SourceEpoch <= snapshot.Epoch
            && binding.SourceGeneration <= snapshot.StateGeneration;
    }

    private sealed record LiveCardBindingAssociation(
        string InstanceId,
        string CallerId,
        string SessionId,
        string LeaseId,
        ulong LeaseEpoch,
        string ProjectId,
        string RunId,
        string EpisodeId,
        string AgentId,
        ulong AuthorityEpoch,
        string ContentManifestId,
        string NativeRunId,
        string SourceIncarnation,
        ulong SourceEpoch,
        ulong SourceGeneration);
}
