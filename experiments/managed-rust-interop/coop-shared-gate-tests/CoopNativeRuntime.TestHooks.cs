// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

// Test-only accessors keep the production composition file unchanged while exercising the
// default cross-profile predicate installed by ModEntry.ConfigureCoopNative.
internal sealed partial class CoopNativeRuntime
{
    internal (int Status, string Response) TestHandle(
        string instanceId, string sessionId, string leaseId, string correlationId,
        string leaseEpoch, string expectedKind, string body) =>
        Handle(instanceId, sessionId, leaseId, correlationId, leaseEpoch, expectedKind, body);

    internal CoopOperationReceipt TestDispatch(CoopLocalActionRequest request) =>
        _host.DispatchLocalAction(request);

    internal bool TestReconcile(string operationId, out CoopOperationReceipt? receipt) =>
        _host.Reconcile(operationId, out receipt);
}
