// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

using System;
using System.Collections.Generic;

// The host-candidate probe compiles the actual RuntimeV3GameplayInterop.cs, but it does not
// load the full game-specific LiveCombatSource or expert implementation. These bounded types
// provide only the symbols needed to compile that production partial class.
internal sealed class LiveCombatSource : IRuntimeV3HostSource
{
    public RuntimeV3GameplayObservation Observe() =>
        throw new InvalidOperationException("bounded expert source stub");

    public IReadOnlyList<LegalActionReference> LegalActions(
        RuntimeV3GameplayObservation observation) => Array.Empty<LegalActionReference>();

    public bool Dispatch(RuntimeV3OperationKey operation, LegalActionReference action) => false;

    public RuntimeV3HostCompletion? Completion(
        RuntimeV3OperationKey operation, LegalActionReference action) => null;
}

internal sealed record RuntimeV4ExpertContext(
    string InstanceId,
    string SessionId,
    string LeaseId,
    ulong LeaseEpoch,
    string CorrelationId);

internal sealed class RuntimeV4ExpertSupport
{
    private int _pendingReads;

    internal static bool PendingForTest { get; set; }

    private RuntimeV4ExpertSupport()
    {
    }

    internal bool HasPendingMutation
    {
        get
        {
            _pendingReads++;
            return PendingForTest;
        }
    }

    internal static RuntimeV4ExpertSupport Unconfigured() => new();

    internal static RuntimeV4ExpertSupport WithHost(
        LiveCombatSource source, IRuntimeV3HostThread thread) => new();

    internal (int Status, string Response) HandleState(
        RuntimeV4ExpertContext context, out int status)
    {
        status = HasPendingMutation ? 409 : 503;
        return (status, "{\"error_code\":\"runtime_v4_expert_host_unavailable\"}");
    }

    internal (int Status, string Response) Handle(
        RuntimeV4ExpertContext context, string body, out int status)
    {
        status = HasPendingMutation ? 409 : 503;
        return (status, "{\"error_code\":\"runtime_v4_expert_host_unavailable\"}");
    }
}
