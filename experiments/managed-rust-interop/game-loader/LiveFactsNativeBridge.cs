// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const int LiveFactsMaxInputBytes = 16 * 1024 * 1024;
    private const int LiveFactsMaxOutputBytes = 262_144;

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int LiveFactsRespond(nint input, uint inputLength,
        nint output, uint outputCapacity, out uint outputLength);

    internal static bool TryProduceLiveFacts(string input, out int status, out string response)
    {
        status = RuntimeUnavailable;
        response = string.Empty;
        if (_nativeLibrary == 0) return false;
        byte[] bytes = Encoding.UTF8.GetBytes(input);
        if (bytes.Length > LiveFactsMaxInputBytes) return false;
        nint source = 0;
        nint output = 0;
        try
        {
            source = Marshal.AllocHGlobal(bytes.Length);
            output = Marshal.AllocHGlobal(LiveFactsMaxOutputBytes);
            Marshal.Copy(bytes, 0, source, bytes.Length);
            var produce = Marshal.GetDelegateForFunctionPointer<LiveFactsRespond>(
                NativeLibrary.GetExport(_nativeLibrary, "sts2_game_mod_live_facts_respond"));
            int result = produce(source, checked((uint)bytes.Length), output,
                LiveFactsMaxOutputBytes, out uint length);
            if (length == 0 || length > LiveFactsMaxOutputBytes) return false;
            byte[] returned = new byte[length];
            Marshal.Copy(output, returned, 0, returned.Length);
            response = new UTF8Encoding(false, true).GetString(returned);
            status = result;
            return true;
        }
        catch (Exception) { return false; }
        finally
        {
            if (output != 0) Marshal.FreeHGlobal(output);
            if (source != 0) Marshal.FreeHGlobal(source);
        }
    }
}
