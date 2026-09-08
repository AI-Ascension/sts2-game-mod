// SPDX-License-Identifier: MIT

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static RuntimeMapV1Support? _runtimeMapV1;

    private static void InitializeRuntimeMapV1()
    {
        _runtimeMapV1 = RuntimeMapV1Support.Unconfigured();
    }

    private static void ConfigureRuntimeMapV1(IRuntimeMapV1HostSource source)
    {
        _runtimeMapV1 = RuntimeMapV1Support.WithHost(source);
    }

    private static (int Status, string Response) ProcessRuntimeMapV1Work(
        RuntimeContext context, string body)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string error))
            return (RuntimeRejected, RuntimeV2PlainError(error));

        RuntimeMapV1Support support = _runtimeMapV1 ?? RuntimeMapV1Support.Unconfigured();
        string response = support.Handle(context.InstanceId, context.SessionId, context.LeaseId,
            context.CorrelationId, context.LeaseEpoch, body, out int status);
        return (status, response);
    }
}
