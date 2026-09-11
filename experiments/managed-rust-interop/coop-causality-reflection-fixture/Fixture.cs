// SPDX-License-Identifier: MIT

namespace MegaCrit.Sts2.Core.Multiplayer.Serialization;

public interface INetMessage { }

public class FixtureMessage : INetMessage { }

public static class MessageTypes
{
    public static void RegisterMessageHandler<T>(Action<T> handler) where T : INetMessage { }
    public static void RegisterMessageHandler<T>(Action<T> handler, int priority) where T : INetMessage { }
}

public class InheritedMessage : FixtureMessage { }

public interface IFixtureMessage : INetMessage { }

public sealed class InterfaceDerivedMessage : IFixtureMessage
{
    public void Overload() { _ = GetHashCode(); }
    private static void Overload(FixtureMessage message) { }
}

public sealed class PrivateStaticMessage : InheritedMessage
{
    static PrivateStaticMessage()
    {
        string? sentinel = Environment.GetEnvironmentVariable("COOP_CAUSALITY_REFLECTION_SENTINEL");
        if (!string.IsNullOrEmpty(sentinel)) File.WriteAllText(sentinel, "executed");
    }

    private static void Hidden(FixtureMessage message)
    {
        MessageTypes.RegisterMessageHandler<FixtureMessage>(static _ => { });
        MessageTypes.RegisterMessageHandler<FixtureMessage>(static _ => { }, 1);
    }
}
