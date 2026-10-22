// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Owned bounded UTF-8 buffers for the fixed native facts codec and producer.</summary>
internal static class GameFactsNativeBridge
{
    internal const int MaxMessageBytes = 262144;
    internal const int MaxSourceBytes = 17 * 1024 * 1024;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int HandleFacts(uint operation, nint request, nuint requestLength,
        nint source, nuint sourceLength, nint correlation, nuint correlationLength,
        nint output, nuint outputCapacity, out nuint outputLength);

    internal static (int Status, string Response) Invoke(
        nint library, uint operation, string request, string source, string correlation)
    {
        if (library == 0)
            return Unavailable();
        if (Encoding.UTF8.GetByteCount(request) > MaxMessageBytes
            || Encoding.UTF8.GetByteCount(source) > MaxSourceBytes
            || Encoding.UTF8.GetByteCount(correlation) is < 1 or > 128)
            return (400, "{\"error_code\":\"game_facts_input_limit\"}");
        nint requestPointer = 0, sourcePointer = 0, correlationPointer = 0, outputPointer = 0;
        try
        {
            byte[] requestBytes = Encoding.UTF8.GetBytes(request);
            byte[] sourceBytes = Encoding.UTF8.GetBytes(source);
            byte[] correlationBytes = Encoding.UTF8.GetBytes(correlation);
            requestPointer = Copy(requestBytes);
            sourcePointer = Copy(sourceBytes);
            correlationPointer = Copy(correlationBytes);
            outputPointer = Marshal.AllocHGlobal(MaxMessageBytes);
            HandleFacts handle = Marshal.GetDelegateForFunctionPointer<HandleFacts>(
                NativeLibrary.GetExport(library, "sts2_game_mod_game_facts_handle"));
            int status = handle(operation, requestPointer, (nuint)requestBytes.Length,
                sourcePointer, (nuint)sourceBytes.Length, correlationPointer,
                (nuint)correlationBytes.Length, outputPointer, MaxMessageBytes,
                out nuint length);
            if (length is 0 or > MaxMessageBytes)
                return Unavailable();
            byte[] response = new byte[(int)length];
            Marshal.Copy(outputPointer, response, 0, response.Length);
            return (status, new UTF8Encoding(false, true).GetString(response));
        }
        catch (Exception)
        {
            return Unavailable();
        }
        finally
        {
            if (outputPointer != 0) Marshal.FreeHGlobal(outputPointer);
            if (correlationPointer != 0) Marshal.FreeHGlobal(correlationPointer);
            if (sourcePointer != 0) Marshal.FreeHGlobal(sourcePointer);
            if (requestPointer != 0) Marshal.FreeHGlobal(requestPointer);
        }
    }

    private static nint Copy(byte[] bytes)
    {
        nint pointer = Marshal.AllocHGlobal(Math.Max(bytes.Length, 1));
        Marshal.Copy(bytes, 0, pointer, bytes.Length);
        return pointer;
    }

    private static (int, string) Unavailable() =>
        (503, "{\"error_code\":\"game_facts_native_unavailable\"}");
}
