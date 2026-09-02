// SPDX-License-Identifier: MIT

using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string RuntimeTokenVariable = "STS2_RUNTIME_TOKEN";
    private const string RuntimePortVariable = "STS2_RUNTIME_PORT";
    private const int RuntimeDefaultPort = 15526;
    private const int RuntimeRequestKindState = 1;
    private const int RuntimeRequestKindAction = 2;
    private const int RuntimeAccepted = 200;
    private const int RuntimeRejected = 409;
    private const int RuntimeUnavailable = 503;
    private const int RuntimeTimeout = 504;
    private static readonly ConcurrentQueue<RuntimeWork> RuntimeQueue = new();
    private static RuntimeRequestCallback? _runtimeRequestCallback;
    private static Action? _runtimePumpCallback;
    private static int _runtimePumpReady;
    private static ulong _runtimeGeneration;
    private static ulong _runtimeActionCount;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int RuntimeStart(ushort port, nint token, nuint tokenLength, ref RuntimeCallbacks callbacks);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int RuntimeRequestCallback(nint request, nint output, nuint outputCapacity, out nuint outputLength);

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
        public RuntimeContext(string instanceId, string callerId, string sessionId, string leaseId, string leaseEpoch, string correlationId)
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
        public System.Threading.ManualResetEventSlim Completed { get; } = new(false);
    }

    private static void StartRuntimeServer(nint nativeLibrary)
    {
        string? token = System.Environment.GetEnvironmentVariable(RuntimeTokenVariable);
        if (string.IsNullOrWhiteSpace(token))
        {
            GD.Print($"{LogPrefix} runtime HTTP listener disabled: {RuntimeTokenVariable} is not set");
            return;
        }
        if (!TryReadPort(out ushort port))
        {
            GD.PrintErr($"{LogPrefix} runtime HTTP listener disabled: {RuntimePortVariable} is invalid");
            return;
        }

        try
        {
            InstallRuntimePump();
            _runtimeRequestCallback = HandleRuntimeRequest;
            var callbacks = new RuntimeCallbacks
            {
                Request = Marshal.GetFunctionPointerForDelegate(_runtimeRequestCallback)
            };
            nint export = NativeLibrary.GetExport(nativeLibrary, "sts2_game_mod_runtime_start");
            RuntimeStart start = Marshal.GetDelegateForFunctionPointer<RuntimeStart>(export);
            byte[] tokenBytes = Encoding.UTF8.GetBytes(token);
            GCHandle tokenHandle = GCHandle.Alloc(tokenBytes, GCHandleType.Pinned);
            try
            {
                int status = start(port, tokenHandle.AddrOfPinnedObject(), (nuint)tokenBytes.Length, ref callbacks);
                if (status == 0)
                {
                    GD.Print($"{LogPrefix} authenticated runtime HTTP listener started on 127.0.0.1:{port}");
                }
                else
                {
                    GD.PrintErr($"{LogPrefix} runtime HTTP listener failed to start: status={status}");
                }
            }
            finally
            {
                tokenHandle.Free();
            }
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} runtime HTTP listener unavailable: {exception.GetType().Name}: {exception.Message}");
        }
    }

    private static bool TryReadPort(out ushort port)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimePortVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            port = RuntimeDefaultPort;
            return true;
        }
        return ushort.TryParse(value, out port) && port > 0;
    }

    private static void InstallRuntimePump()
    {
        if (Volatile.Read(ref _runtimePumpReady) != 0)
        {
            return;
        }
        if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
        {
            throw new InvalidOperationException("SceneTree root is unavailable for runtime dispatch");
        }
        _runtimePumpCallback = ProcessRuntimeQueue;
        tree.ProcessFrame += _runtimePumpCallback;
        Volatile.Write(ref _runtimePumpReady, 1);
    }

    private static int HandleRuntimeRequest(nint requestPointer, nint output, nuint outputCapacity, out nuint outputLength)
    {
        outputLength = 0;
        try
        {
            if (requestPointer == 0 || output == 0 || outputCapacity == 0)
            {
                return RuntimeUnavailable;
            }
            NativeRuntimeRequest native = Marshal.PtrToStructure<NativeRuntimeRequest>(requestPointer);
            RuntimeContext context = new(
                ReadNativeText(native.InstanceId, native.InstanceIdLength),
                ReadNativeText(native.CallerId, native.CallerIdLength),
                ReadNativeText(native.SessionId, native.SessionIdLength),
                ReadNativeText(native.LeaseId, native.LeaseIdLength),
                ReadNativeText(native.LeaseEpoch, native.LeaseEpochLength),
                ReadNativeText(native.CorrelationId, native.CorrelationIdLength));
            string body = ReadNativeText(native.Body, native.BodyLength);
            RuntimeWork work = new(native.Kind, context, body);
            if (Volatile.Read(ref _runtimePumpReady) == 0)
            {
                work.Status = RuntimeUnavailable;
                work.Response = RuntimeError(context, native.Kind, "runtime_pump_unavailable");
            }
            else
            {
                RuntimeQueue.Enqueue(work);
                if (!work.Completed.Wait(TimeSpan.FromSeconds(5)))
                {
                    work.Status = RuntimeTimeout;
                    work.Response = RuntimeError(context, native.Kind, "main_thread_timeout");
                }
            }
            return WriteNativeResponse(work.Status, work.Response, output, outputCapacity, out outputLength);
        }
        catch (Exception exception)
        {
            GD.PrintErr($"{LogPrefix} runtime callback failed: {exception.GetType().Name}: {exception.Message}");
            return RuntimeUnavailable;
        }
    }

    private static void ProcessRuntimeQueue()
    {
        for (int index = 0; index < 16 && RuntimeQueue.TryDequeue(out RuntimeWork? work); index++)
        {
            try
            {
                (work.Status, work.Response) = ProcessRuntimeWork(work);
            }
            catch (Exception exception)
            {
                work.Status = RuntimeUnavailable;
                work.Response = RuntimeError(work.Context, work.Kind, "main_thread_exception");
                GD.PrintErr($"{LogPrefix} runtime main-thread request failed: {exception.GetType().Name}: {exception.Message}");
            }
            finally
            {
                work.Completed.Set();
            }
        }
    }

    private static string ReadNativeText(nint pointer, nuint length)
    {
        if (length == 0)
        {
            return string.Empty;
        }
        if (pointer == 0 || length > 16 * 1024)
        {
            throw new InvalidOperationException("native text pointer is invalid");
        }
        int byteLength = checked((int)length);
        byte[] bytes = new byte[byteLength];
        Marshal.Copy(pointer, bytes, 0, byteLength);
        return Encoding.UTF8.GetString(bytes);
    }

    private static int WriteNativeResponse(int status, string response, nint output, nuint outputCapacity, out nuint outputLength)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(response);
        if (bytes.Length > (long)outputCapacity || bytes.Length > 64 * 1024)
        {
            outputLength = 0;
            return RuntimeUnavailable;
        }
        Marshal.Copy(bytes, 0, output, bytes.Length);
        outputLength = (nuint)bytes.Length;
        return status;
    }
}
