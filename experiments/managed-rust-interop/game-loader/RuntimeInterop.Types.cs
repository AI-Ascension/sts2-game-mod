// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string FormatEndpoint(string bindAddress, ushort port) =>
        bindAddress.Contains(':') ? $"[{bindAddress}]:{port}" : $"{bindAddress}:{port}";

    [StructLayout(LayoutKind.Sequential)]
    private struct RuntimeCallbacks
    {
        public nint Request;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeRuntimeRequest
    {
        public uint Kind;
        public nint InstanceId;
        public nuint InstanceIdLength;
        public nint CallerId;
        public nuint CallerIdLength;
        public nint SessionId;
        public nuint SessionIdLength;
        public nint LeaseId;
        public nuint LeaseIdLength;
        public nint LeaseEpoch;
        public nuint LeaseEpochLength;
        public nint CorrelationId;
        public nuint CorrelationIdLength;
        public nint Body;
        public nuint BodyLength;
    }

    private readonly struct RuntimeContext
    {
        public RuntimeContext(
            string instanceId,
            string callerId,
            string sessionId,
            string leaseId,
            string leaseEpoch,
            string correlationId)
        {
            InstanceId = instanceId;
            CallerId = callerId;
            SessionId = sessionId;
            LeaseId = leaseId;
            LeaseEpoch = leaseEpoch;
            CorrelationId = correlationId;
        }

        public string InstanceId { get; }
        public string CallerId { get; }
        public string SessionId { get; }
        public string LeaseId { get; }
        public string LeaseEpoch { get; }
        public string CorrelationId { get; }
    }

    private sealed class RuntimeWork
    {
        public RuntimeWork(uint kind, RuntimeContext context, string body)
        {
            Kind = kind;
            Context = context;
            Body = body;
        }

        public uint Kind { get; }
        public RuntimeContext Context { get; }
        public string Body { get; }
        public int Status { get; set; } = RuntimeUnavailable;
        public string Response { get; set; } = "{\"error_code\":\"runtime_unavailable\"}";
        public ManualResetEventSlim Completed { get; } = new(false);
    }
}
