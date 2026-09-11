// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;
using MegaCrit.Sts2.Core.Multiplayer.Serialization;

namespace AiAscension.Sts2GameMod.OpaqueOperationNativeMessageHostShape;

internal static class Program
{
    private static void Main()
    {
        INetMessage message = new OpaqueOperationNativeMessage();
        IPacketSerializable serializable = (IPacketSerializable)message;
        Console.WriteLine("Exact-host opaque native-message interface shape compiled: "
            + serializable.GetType().FullName);
    }
}
