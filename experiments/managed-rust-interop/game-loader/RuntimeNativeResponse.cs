// SPDX-License-Identifier: MIT

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static int WriteNativeResponse(
        int status,
        string response,
        uint requestKind,
        string correlationId,
        nint output,
        nuint outputCapacity,
        out nuint outputLength)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(response);
        int maximumBytes = requestKind == RuntimeRequestKindContentManifest
            ? ContentManifestWireContract.MaxMessageBytes
            : RuntimeMapV1Contract.MaxMessageBytes;
        if (bytes.Length > (long)outputCapacity || bytes.Length > maximumBytes)
        {
            if (requestKind == RuntimeRequestKindContentManifest)
            {
                status = 413;
                bytes = Encoding.UTF8.GetBytes(ContentManifestError(
                    correlationId, "result_limit_exceeded", "serialized_payload_too_large"));
                if (bytes.Length <= (long)outputCapacity)
                {
                    Marshal.Copy(bytes, 0, output, bytes.Length);
                    outputLength = (nuint)bytes.Length;
                    return status;
                }
            }
            outputLength = 0;
            return RuntimeUnavailable;
        }
        Marshal.Copy(bytes, 0, output, bytes.Length);
        outputLength = (nuint)bytes.Length;
        return status;
    }
}
