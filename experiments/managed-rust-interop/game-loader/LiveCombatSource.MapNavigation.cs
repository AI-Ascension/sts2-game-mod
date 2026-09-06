// SPDX-License-Identifier: MIT

using System;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private async Task NavigateCampaignMapAsync(MoveToMapCoordAction queued)
    {
        RunManager.Instance.ActionQueueSynchronizer.RequestEnqueue(queued);
        // A queue task can complete outside the host thread. Wait through host frames
        // before observing its result or touching the destination's native controls.
        for (int frame = 0; frame < 1800; frame++)
        {
            RequireThread();
            if (queued.CompletionTask.IsCompleted)
            {
                await queued.CompletionTask;
                RequireThread();
                await OpenEnteredShopAsync();
                return;
            }
            await WaitCampaignFrameAsync();
        }
        throw new InvalidOperationException("native map navigation exceeded its frame bound");
    }
}
