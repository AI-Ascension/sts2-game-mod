// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

// Test-only accessors keep the production composition file unchanged while exercising the
// default cross-profile predicate installed by ModEntry.ConfigureCoopNative.
internal sealed partial class CoopNativeRuntime
{
    internal CoopOperationReceipt TestDispatch(CoopLocalActionRequest request) =>
        _host.DispatchLocalAction(request);

    internal bool TestReconcile(string operationId, out CoopOperationReceipt? receipt) =>
        _host.Reconcile(operationId, out receipt);
}
