// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplayRecoveryStore
{
    private sealed class FencePayload
    {
        public string DeploymentId { get; set; } = string.Empty;
        public string InstanceId { get; set; } = string.Empty;
        public string InstanceIncarnation { get; set; } = string.Empty;
        public string BootId { get; set; } = string.Empty;
        public ulong AuthorityGeneration { get; set; }
        public string LeaseId { get; set; } = string.Empty;
        public ulong LeaseEpoch { get; set; }
        public string HostFenceId { get; set; } = string.Empty;
        public ulong FenceGeneration { get; set; }
        public string ExpiresAt { get; set; } = string.Empty;
        public string ReleaseDigest { get; set; } = string.Empty;
        public string ConfigDigest { get; set; } = string.Empty;
        public string ProfileDigest { get; set; } = string.Empty;
        public string RuntimeV3SchemaDigest { get; set; } = string.Empty;

        public FencePayload() { }

        internal FencePayload(RuntimeV3HostFence fence, RuntimeV3RecoveryRelease release)
        {
            DeploymentId = fence.DeploymentId;
            InstanceId = fence.InstanceId;
            InstanceIncarnation = fence.InstanceIncarnation;
            BootId = fence.BootId;
            AuthorityGeneration = fence.AuthorityGeneration;
            LeaseId = fence.LeaseId;
            LeaseEpoch = fence.LeaseEpoch;
            HostFenceId = fence.HostFenceId;
            FenceGeneration = fence.FenceGeneration;
            ExpiresAt = fence.ExpiresAt.ToUniversalTime().ToString("O");
            ReleaseDigest = release.ReleaseDigest;
            ConfigDigest = release.ConfigDigest;
            ProfileDigest = release.ProfileDigest;
            RuntimeV3SchemaDigest = release.RuntimeV3SchemaDigest;
        }

        internal RuntimeV3HostFence ToFence() =>
            new(DeploymentId, InstanceId, InstanceIncarnation, BootId, AuthorityGeneration,
                LeaseId, LeaseEpoch, HostFenceId, FenceGeneration,
                DateTimeOffset.Parse(ExpiresAt, null, System.Globalization.DateTimeStyles.RoundtripKind));

        internal RuntimeV3RecoveryRelease ToRelease() =>
            new(ReleaseDigest, ConfigDigest, ProfileDigest, RuntimeV3SchemaDigest);
    }
}
