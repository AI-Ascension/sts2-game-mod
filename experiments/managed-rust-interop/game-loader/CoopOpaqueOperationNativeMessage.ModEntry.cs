// SPDX-License-Identifier: MIT

using System;
using MegaCrit.Sts2.Core.Multiplayer.Game;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static IHostAdmittedOpaqueOperationDispatcher? _opaqueOperationDispatcher;
    private static Func<ulong, bool>? _opaqueOperationAuthenticatedHost;
    private static Action<OpaqueOperationNativeMessage>? _opaqueOperationReplySink;
    private static OpaqueOperationNativeMessageAdapter? _opaqueOperationAdapter;
    private static bool _opaqueOperationServiceHooked;

    /// <summary>
    /// Explicitly binds the opaque operation transport to the first-party native service once
    /// that service is available. The caller supplies host admission and client-host identity;
    /// this method never substitutes a client claim for a host settlement witness.
    /// </summary>
    internal static void ConfigureOpaqueOperationNativeTransport(
        IHostAdmittedOpaqueOperationDispatcher? hostDispatcher,
        Func<ulong, bool> isAuthenticatedHost,
        Action<OpaqueOperationNativeMessage>? replySink = null)
    {
        _opaqueOperationDispatcher = hostDispatcher;
        _opaqueOperationAuthenticatedHost = isAuthenticatedHost
            ?? throw new ArgumentNullException(nameof(isAuthenticatedHost));
        _opaqueOperationReplySink = replySink;
        if (!_opaqueOperationServiceHooked)
        {
            NativeCoopSessionController.ActiveServiceChanged += BindOpaqueOperationService;
            _opaqueOperationServiceHooked = true;
        }
        BindOpaqueOperationService(NativeCoopSessionController.ActiveService);
    }

    private static void BindOpaqueOperationService(INetGameService? service)
    {
        _opaqueOperationAdapter?.Dispose();
        _opaqueOperationAdapter = null;
        if (service is null || _opaqueOperationAuthenticatedHost is null)
            return;
        _opaqueOperationAdapter = new OpaqueOperationNativeMessageAdapter(service,
            _opaqueOperationDispatcher, _opaqueOperationAuthenticatedHost, _opaqueOperationReplySink);
        _opaqueOperationAdapter.Register();
    }
}
