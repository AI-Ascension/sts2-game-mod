// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const int ContentManifestNativeOutputBytes = 16 * 1024 * 1024;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int ContentManifestProduce(
        nint input,
        nuint inputLength,
        nint correlation,
        nuint correlationLength,
        nint output,
        nuint outputCapacity,
        out nuint outputLength);

    internal static bool TryProduceContentManifest(
        string input,
        string correlationId,
        out int status,
        out string response)
    {
        status = RuntimeUnavailable;
        response = string.Empty;
        if (_nativeLibrary == 0)
            return false;

        byte[] inputBytes = Encoding.UTF8.GetBytes(input);
        if (inputBytes.Length > ContentManifestWireContract.MaxMessageBytes)
            return false;
        byte[] correlationBytes = Encoding.UTF8.GetBytes(correlationId);
        nint inputPointer = 0;
        nint correlationPointer = 0;
        nint outputPointer = 0;
        try
        {
            inputPointer = Marshal.AllocHGlobal(inputBytes.Length);
            correlationPointer = Marshal.AllocHGlobal(correlationBytes.Length);
            outputPointer = Marshal.AllocHGlobal(ContentManifestNativeOutputBytes);
            Marshal.Copy(inputBytes, 0, inputPointer, inputBytes.Length);
            Marshal.Copy(correlationBytes, 0, correlationPointer, correlationBytes.Length);
            nint export = NativeLibrary.GetExport(
                _nativeLibrary, "sts2_game_mod_content_manifest_produce");
            ContentManifestProduce produce =
                Marshal.GetDelegateForFunctionPointer<ContentManifestProduce>(export);
            int nativeStatus = produce(
                inputPointer,
                (nuint)inputBytes.Length,
                correlationPointer,
                (nuint)correlationBytes.Length,
                outputPointer,
                ContentManifestNativeOutputBytes,
                out nuint outputLength);
            if (outputLength > ContentManifestNativeOutputBytes)
                return false;
            byte[] responseBytes = new byte[(int)outputLength];
            if (responseBytes.Length > 0)
                Marshal.Copy(outputPointer, responseBytes, 0, responseBytes.Length);
            status = nativeStatus;
            response = Encoding.UTF8.GetString(responseBytes);
            return true;
        }
        catch (Exception)
        {
            status = RuntimeUnavailable;
            response = string.Empty;
            return false;
        }
        finally
        {
            if (outputPointer != 0) Marshal.FreeHGlobal(outputPointer);
            if (correlationPointer != 0) Marshal.FreeHGlobal(correlationPointer);
            if (inputPointer != 0) Marshal.FreeHGlobal(inputPointer);
        }
    }
}
