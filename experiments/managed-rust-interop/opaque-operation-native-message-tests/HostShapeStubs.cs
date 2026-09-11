// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace MegaCrit.Sts2.Core.Logging
{
    public enum LogLevel
    {
        Trace
    }
}

namespace MegaCrit.Sts2.Core.Multiplayer.Transport
{
    public enum NetTransferMode
    {
        Unreliable,
        Reliable,
        Ordered
    }
}

namespace MegaCrit.Sts2.Core.Multiplayer.Serialization
{
    public interface INetMessage
    {
        bool ShouldBroadcast { get; }
        MegaCrit.Sts2.Core.Multiplayer.Transport.NetTransferMode Mode { get; }
        MegaCrit.Sts2.Core.Logging.LogLevel LogLevel { get; }
        bool ShouldBuffer { get; }
    }

    public interface IPacketSerializable
    {
        void Serialize(PacketWriter writer);
        void Deserialize(PacketReader reader);
    }

    public sealed class PacketWriter
    {
        internal List<object> Values { get; } = new();

        public void WriteByte(byte value) => Values.Add(value);
        public void WriteInt(int value) => Values.Add(value);
        public void WriteBytes(byte[] value, int length)
        {
            byte[] copy = new byte[length];
            Array.Copy(value, copy, length);
            Values.Add(copy);
        }
    }

    public sealed class PacketReader
    {
        private readonly Queue<object> _values;

        internal PacketReader(IEnumerable<object> values) => _values = new Queue<object>(values);

        public byte ReadByte() => (byte)_values.Dequeue();
        public int ReadInt() => (int)_values.Dequeue();
        public void ReadBytes(byte[] destination, int length)
        {
            byte[] source = (byte[])_values.Dequeue();
            Array.Copy(source, destination, length);
        }
    }
}

namespace MegaCrit.Sts2.Core.Multiplayer.Game
{
#pragma warning disable CA1711 // Must mirror the exact-host delegate name.
    public delegate void MessageHandlerDelegate<T>(T message, ulong senderId)
        where T : MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessage;
#pragma warning restore CA1711

    public interface INetGameService
    {
        void RegisterMessageHandler<T>(MessageHandlerDelegate<T> handler)
            where T : MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessage;
        void UnregisterMessageHandler<T>(MessageHandlerDelegate<T> handler)
            where T : MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessage;
        void SendMessage<T>(T message) where T : MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessage;
        void SendMessage<T>(T message, ulong peerId)
            where T : MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessage;
    }

    public interface INetHostGameService : INetGameService
    {
    }
}
