// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.RestSite;
using MegaCrit.Sts2.Core.Models.Relics;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Nodes.CommonUi;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static bool IsNativeWitnessOption(RestSiteOption option) =>
        WireRestOptionId(option.OptionId) is "kindle" or "lift";

    private async Task ExecuteNativeImmediateRestAsync(ExpertRestPending pending)
    {
        Player player = CurrentPlayer()
            ?? throw new InvalidOperationException("native rest option has no local player");
        string optionId = WireRestOptionId(pending.Option.OptionId)!;
        PumpkinCandle? kindle = optionId == "kindle"
            ? player.Relics.OfType<PumpkinCandle>().FirstOrDefault() : null;
        Girya? lift = optionId == "lift"
            ? player.Relics.OfType<Girya>().FirstOrDefault() : null;
        int before = optionId == "kindle" ? kindle?.KindleCount ?? 0 : lift?.TimesLifted ?? 0;
        var expectation = new ExpertRestNativeExpectation(optionId, player.NetId, before, kindle, lift);
        pending.NativeExpectation = expectation;

        RestSiteSynchronizer synchronizer = RunManager.Instance.RestSiteSynchronizer;
        Action<RestSiteOption, bool, ulong> callback = (option, success, playerId) =>
        {
            if (pending.NativeCallback is not null
                || !ReferenceEquals(option, pending.Option)
                || playerId != expectation.PlayerId)
                return;
            pending.NativeCallback = new ExpertRestNativeCallback(
                WireRestOptionId(option.OptionId) ?? string.Empty, success, playerId);
        };
        synchronizer.AfterPlayerOptionChosen += callback;
        try
        {
            pending.Button.ForceClick();
            await WaitImmediateRestSettledAsync(pending);
        }
        finally
        {
            synchronizer.AfterPlayerOptionChosen -= callback;
        }
    }

    private async Task WaitImmediateRestSettledAsync(ExpertRestPending pending)
    {
        for (int frame = 0; frame < 600; frame++)
        {
            await WaitCampaignFrameAsync();
            RequireThread();
            NRestSiteRoom? room = CurrentRestSite();
            if (room is null) return;
            NRestSiteButton? current = room.GetButtonForOption(pending.Option);
            if (current is null && Clickable(room.ProceedButton)) return;
            if (current is not null && !ReferenceEquals(current, pending.Button)
                && Clickable(room.ProceedButton)) return;
        }
        throw new InvalidOperationException("native rest option did not settle");
    }

    private static RuntimeV4ExpertRestEffectWitness? NativeImmediateWitness(
        ExpertRestPending pending,
        RuntimeV4ExpertGameplayObservation after)
    {
        if (pending.NativeExpectation is not { } expectation
            || pending.NativeCallback is not { } callback
            || callback.OptionId != expectation.OptionId
            || !callback.Success
            || callback.PlayerId != expectation.PlayerId)
            return null;

        if (expectation.OptionId == "kindle")
        {
            if (expectation.Kindle is not { } kindle || expectation.Before < 0)
                return null;
            int current = kindle.KindleCount;
            if (current < 0 || (long)current - expectation.Before != 5)
                return null;
            return new RuntimeV4ExpertRestEffectWitness(
                "kindle_applied", pending.Operation, "kindle", after.Generation,
                new RuntimeV4ExpertRestNativeEvidence(
                    pending.Operation.OperationId, after.StateId));
        }

        if (expectation.Lift is not { } lift || expectation.Before is < 0 or >= 3)
            return null;
        int lifted = lift.TimesLifted;
        if (lifted is < 0 or > 3 || (long)lifted - expectation.Before != 1)
            return null;
        return new RuntimeV4ExpertRestEffectWitness(
            "lift_applied", pending.Operation, "lift", after.Generation,
            new RuntimeV4ExpertRestStatEvidence(
                "times_lifted", expectation.Before, lifted));
    }
}
