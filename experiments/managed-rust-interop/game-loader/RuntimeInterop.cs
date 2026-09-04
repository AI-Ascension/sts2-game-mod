// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string RuntimeTokenVariable = "STS2_RUNTIME_TOKEN";
    private const string RuntimePortVariable = "STS2_RUNTIME_PORT";
    private const string RuntimeBindAddressVariable = "STS2_RUNTIME_BIND_ADDRESS";
    private const int RuntimeRequestKindState = 1;
    private const int RuntimeRequestKindAction = 2;
    private const int RuntimeAccepted = 200;
    private const int RuntimeRejected = 409;
    private const int RuntimeUnavailable = 503;
    private static readonly RuntimeDispatchQueue<RuntimeWork> RuntimeQueue = new(64);
    private static RuntimeRequestCallback? _runtimeRequestCallback;
    private static Action? _runtimePumpCallback;
    private static int _runtimePumpReady;
    private static ulong _runtimeGeneration;
    private static ulong _runtimeActionCount;
    private static string _runtimeListenerStatus = "Not started";

    internal static string RuntimeAuthenticationStatus =>
        string.IsNullOrWhiteSpace(System.Environment.GetEnvironmentVariable(RuntimeTokenVariable))
            ? "Token missing"
            : "Token configured";

    internal static string RuntimeListenerStatus => _runtimeListenerStatus;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int RuntimeStart(
        ushort port,
        nint bindAddress,
        nuint bindAddressLength,
        nint token,
        nuint tokenLength,
        ref RuntimeCallbacks callbacks);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int RuntimeRequestCallback(nint request, nint output, nuint outputCapacity, out nuint outputLength);

    private static void StartRuntimeServer(nint nativeLibrary)
    {
        if (!StandaloneProfileSettings.RuntimeEnabled && !RuntimeSessionLaunchEnabled())
        {
            _runtimeListenerStatus = "Disabled in settings";
            GD.Print($"{LogPrefix} runtime HTTP listener disabled in settings");
            return;
        }

        string? token = System.Environment.GetEnvironmentVariable(RuntimeTokenVariable);
        if (string.IsNullOrWhiteSpace(token))
        {
            _runtimeListenerStatus = "Disabled: authentication token missing";
            GD.Print($"{LogPrefix} runtime HTTP listener disabled: {RuntimeTokenVariable} is not set");
            return;
        }
        if (!TryReadPort(out ushort port))
        {
            _runtimeListenerStatus = "Disabled: invalid network port";
            GD.PrintErr($"{LogPrefix} runtime HTTP listener disabled: {RuntimePortVariable} is invalid");
            return;
        }
        if (!TryReadBindAddress(out string bindAddress))
        {
            _runtimeListenerStatus = "Disabled: invalid bind address";
            GD.PrintErr($"{LogPrefix} runtime HTTP listener disabled: {RuntimeBindAddressVariable} is invalid");
            return;
        }

        try
        {
            _runtimeListenerStatus = $"Starting on {FormatEndpoint(bindAddress, port)}";
            InstallRuntimePump();
            _runtimeRequestCallback = HandleRuntimeRequest;
            var callbacks = new RuntimeCallbacks
            {
                Request = Marshal.GetFunctionPointerForDelegate(_runtimeRequestCallback)
            };
            nint export = NativeLibrary.GetExport(nativeLibrary, "sts2_game_mod_runtime_start");
            RuntimeStart start = Marshal.GetDelegateForFunctionPointer<RuntimeStart>(export);
            byte[] bindAddressBytes = Encoding.UTF8.GetBytes(bindAddress);
            byte[] tokenBytes = Encoding.UTF8.GetBytes(token);
            GCHandle bindAddressHandle = GCHandle.Alloc(bindAddressBytes, GCHandleType.Pinned);
            GCHandle tokenHandle = GCHandle.Alloc(tokenBytes, GCHandleType.Pinned);
            try
            {
                int status = start(
                    port,
                    bindAddressHandle.AddrOfPinnedObject(),
                    (nuint)bindAddressBytes.Length,
                    tokenHandle.AddrOfPinnedObject(),
                    (nuint)tokenBytes.Length,
                    ref callbacks);
                if (status == 0)
                {
                    _runtimeListenerStatus = $"Listening on {FormatEndpoint(bindAddress, port)}";
                    GD.Print($"{LogPrefix} authenticated runtime HTTP listener started on {FormatEndpoint(bindAddress, port)}");
                }
                else
                {
                    _runtimeListenerStatus = $"Failed to start listener (status {status})";
                    GD.PrintErr($"{LogPrefix} runtime HTTP listener failed to start: status={status}");
                }
            }
            finally
            {
                tokenHandle.Free();
                bindAddressHandle.Free();
            }
        }
        catch (Exception exception)
        {
            _runtimeListenerStatus = $"Unavailable: {exception.GetType().Name}";
            GD.PrintErr($"{LogPrefix} runtime HTTP listener unavailable: {exception.GetType().Name}: {exception.Message}");
        }
    }

    private static bool TryReadPort(out ushort port)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimePortVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            int configuredPort = StandaloneProfileSettings.RuntimePort;
            if (configuredPort <= 0 || configuredPort > ushort.MaxValue)
            {
                port = 0;
                return false;
            }

            port = (ushort)configuredPort;
            return true;
        }
        return ushort.TryParse(value, out port) && port > 0;
    }

    private static bool TryReadBindAddress(out string bindAddress)
    {
        string? value = System.Environment.GetEnvironmentVariable(RuntimeBindAddressVariable);
        if (string.IsNullOrWhiteSpace(value))
        {
            value = StandaloneProfileSettings.RuntimeBindAddress;
        }

        value = value.Trim();
        if (!StandaloneProfileSettings.IsValidRuntimeBindAddress(value))
        {
            bindAddress = string.Empty;
            return false;
        }

        bindAddress = value;
        return true;
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
                return WriteNativeResponse(RuntimeUnavailable,
                    "{\"error_code\":\"runtime_pump_unavailable\"}", output, outputCapacity, out outputLength);
            }
            else
            {
                var pending = RuntimeQueue.Enqueue(work);
                if (pending == null)
                {
                    return WriteNativeResponse(RuntimeUnavailable,
                        "{\"error_code\":\"runtime_queue_full\"}", output, outputCapacity, out outputLength);
                }
                var response = RuntimeQueue.Wait(pending, TimeSpan.FromSeconds(5));
                return WriteNativeResponse(response.Status, response.Response, output, outputCapacity, out outputLength);
            }
        }
        catch (Exception)
        {
            GD.PrintErr($"{LogPrefix} runtime callback failed");
            return RuntimeUnavailable;
        }
    }

    private static void ProcessRuntimeQueue()
    {
        for (int index = 0; index < 16 && RuntimeQueue.ProcessOne(ExecuteRuntimeWork); index++)
        {
        }
    }

    private static (int Status, string Response) ExecuteRuntimeWork(RuntimeWork work)
    {
        try
        {
            return ProcessRuntimeWork(work);
        }
        catch (Exception)
        {
            GD.PrintErr($"{LogPrefix} runtime main-thread request failed");
            return (RuntimeUnavailable, "{\"error_code\":\"main_thread_outcome_unknown\"}");
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
