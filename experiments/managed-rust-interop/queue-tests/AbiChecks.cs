// SPDX-License-Identifier: MIT

using System;
using System.Reflection;
using System.Runtime.InteropServices;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    internal static void CheckAbiBoundary()
    {
        RequireAbi(Marshal.SizeOf<RuntimeCallbacks>() == IntPtr.Size, "callback table width");
        RequireAbi(Marshal.SizeOf<NativeRuntimeRequest>() == 15 * IntPtr.Size, "request width");
        RequireAbi(typeof(NativeRuntimeRequest).GetField(nameof(NativeRuntimeRequest.Kind))?.FieldType == typeof(uint),
            "fixed-width request kind");
        RequireAbi(Marshal.OffsetOf<NativeRuntimeRequest>(nameof(NativeRuntimeRequest.Kind)) == 0, "kind offset");
        string[] fields =
        [
            nameof(NativeRuntimeRequest.InstanceId), nameof(NativeRuntimeRequest.InstanceIdLength),
            nameof(NativeRuntimeRequest.CallerId), nameof(NativeRuntimeRequest.CallerIdLength),
            nameof(NativeRuntimeRequest.SessionId), nameof(NativeRuntimeRequest.SessionIdLength),
            nameof(NativeRuntimeRequest.LeaseId), nameof(NativeRuntimeRequest.LeaseIdLength),
            nameof(NativeRuntimeRequest.LeaseEpoch), nameof(NativeRuntimeRequest.LeaseEpochLength),
            nameof(NativeRuntimeRequest.CorrelationId), nameof(NativeRuntimeRequest.CorrelationIdLength),
            nameof(NativeRuntimeRequest.Body), nameof(NativeRuntimeRequest.BodyLength)
        ];
        for (int index = 0; index < fields.Length; index++)
        {
            RequireAbi(Marshal.OffsetOf<NativeRuntimeRequest>(fields[index]) == (index + 1) * IntPtr.Size,
                "request field offset");
        }
        RequireAbi(typeof(RuntimeRequestCallback).GetCustomAttribute<UnmanagedFunctionPointerAttribute>()?
            .CallingConvention == CallingConvention.Cdecl, "callback calling convention");
        RequireAbi(typeof(RuntimeStart).GetCustomAttribute<UnmanagedFunctionPointerAttribute>()?
            .CallingConvention == CallingConvention.Cdecl, "start calling convention");

        nint output = Marshal.AllocHGlobal(8);
        nint request = Marshal.AllocHGlobal(Marshal.SizeOf<NativeRuntimeRequest>());
        try
        {
            Marshal.WriteByte(output, 0x5a);
            RequireAbi(HandleRuntimeRequest(0, output, 8, out nuint length) == 503 && length == 0,
                "null request rejected before dereference");
            RequireAbi(HandleRuntimeRequest(request, 0, 8, out length) == 503 && length == 0,
                "null output rejected before dereference");
            RequireAbi(HandleRuntimeRequest(request, output, 0, out length) == 503 && length == 0,
                "zero capacity rejected before dereference");
            var oversized = new NativeRuntimeRequest { InstanceId = 1, InstanceIdLength = 16 * 1024 + 1 };
            Marshal.StructureToPtr(oversized, request, false);
            RequireAbi(HandleRuntimeRequest(request, output, 8, out length) == 503 && length == 0,
                "oversized text rejected before pointer read, exception contained");
            RequireAbi(WriteNativeResponse(200, "é", output, 1, out length) == 503 && length == 0,
                "UTF-8 byte capacity enforced");
            RequireAbi(Marshal.ReadByte(output) == 0x5a, "rejected response preserves caller bytes");
            RequireAbi(WriteNativeResponse(200, "é", output, 2, out length) == 200 && length == 2,
                "exact UTF-8 byte capacity accepted");
            RequireAbi(ReadNativeText(output, length) == "é", "owned UTF-8 roundtrip");
        }
        finally
        {
            Marshal.FreeHGlobal(request);
            Marshal.FreeHGlobal(output);
        }
        Console.WriteLine("Managed ABI layout and 8 invalid/bounded buffer checks passed; exact-host ABI unverified.");
    }

    private static void RequireAbi(bool condition, string name)
    {
        if (!condition) throw new InvalidOperationException(name);
    }
}
