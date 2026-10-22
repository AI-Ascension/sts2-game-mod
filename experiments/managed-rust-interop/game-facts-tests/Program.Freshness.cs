// SPDX-License-Identifier: MIT

using System;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static void SourceCaptureAndFreshness()
    {
        var fixture = new BoundaryFixture();
        fixture.Player.PlayerCombatState!.Energy = 300;
        fixture.Card.EnergyCost.Value = 270;
        LiveCardCapturedSnapshot snapshot = fixture.Reader.Read("instance", "manifest");
        Check(snapshot.Available && snapshot.PlayerEnergy.Value == 300
            && snapshot.Cards[0].ResolvedCost.Value == 270
            && snapshot.Cards[0].FixedEnergyCost.Value && snapshot.ModeId == "standard",
            "production capture preserves raw energy, cost, fixedness and mode");
        LiveCardCapturedSnapshot retained = fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch);
        Check(ReferenceEquals(snapshot, retained) && retained.Epoch == snapshot.Epoch,
            "current retained read never advances the capture epoch");
        Check(RunManager.Instance.State!.Rng.Draws == 0 && fixture.Card.EnergyCost.Reads > 0,
            "capture reads existing fields without advancing the controlled game RNG");

        fixture.Player.PlayerCombatState.Energy++;
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "raw energy changes invalidate even when compact observation generation is unchanged");
        snapshot = fixture.Reader.Read("instance", "manifest");
        fixture.Card.EnergyCost.Value++;
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "raw cost changes invalidate independently of compact byte values");
        snapshot = fixture.Reader.Read("instance", "manifest");
        fixture.Source.Generation++;
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "changed gameplay generation invalidates the retained capture");
        snapshot = fixture.Reader.Read("instance", "manifest");
        RunManager.Instance.State = new();
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "a different native run with the same seed is rejected");
        snapshot = fixture.Reader.Read("instance", "manifest");
        fixture.Player.PlayerCombatState.Hand.Cards[0] = new();
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "replacing a same-definition native card cannot inherit its occurrence identity");
        snapshot = fixture.Reader.Read("instance", "manifest");
        MegaCrit.Sts2.Core.Saves.SaveManager.Instance.CurrentProfileId = 2;
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "profile slot change invalidates the retained source");
        snapshot = fixture.Reader.Read("instance", "manifest");
        MegaCrit.Sts2.Core.Saves.SaveManager.Instance.Progress = new();
        Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
            "same-slot profile replacement invalidates by native object identity");
    }

    private static void FailedReadsCannotRetainOlderValues()
    {
        foreach (string failure in new[] { "identity", "queue", "deferred", "invalidate", "retained" })
        {
            var fixture = new BoundaryFixture();
            LiveCardCapturedSnapshot snapshot = fixture.Reader.Read("instance", "manifest");
            switch (failure)
            {
                case "identity": fixture.Reader.Read(string.Empty, "manifest"); break;
                case "queue": fixture.Thread.Fail = true; fixture.Reader.Read("instance", "manifest"); break;
                case "deferred": fixture.Thread.Defer = true; fixture.Reader.Read("instance", "manifest"); break;
                case "invalidate": fixture.Thread.Fail = true; fixture.Reader.Invalidate(); break;
                default:
                    fixture.Thread.Fail = true;
                    fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch);
                    break;
            }
            fixture.Thread.Fail = false; fixture.Thread.Defer = false;
            Check(!fixture.Reader.ReadRetained("instance", "manifest", snapshot.Epoch).Available,
                failure + " failure clears the older retained value");
            if (failure == "invalidate")
            {
                LiveCardCapturedSnapshot next = fixture.Reader.Read("instance", "manifest");
                Check(next.Available && next.SourceIncarnation != snapshot.SourceIncarnation
                    && next.Cards[0].InstanceId != snapshot.Cards[0].InstanceId,
                    "failed invalidation retires old handles before the next successful capture");
            }
        }
    }

    private static void CaptureRefusalsAreExplicit()
    {
        var fixture = new BoundaryFixture();
        fixture.Card.HasEnergyCostX = null;
        fixture.Card.EnergyCost.Value = -1;
        fixture.Player.PlayerCombatState!.Energy = -1;
        LiveCardCapturedSnapshot snapshot = fixture.Reader.Read("instance", "manifest");
        Check(snapshot.Available && snapshot.PlayerEnergy.Status == LiveCardFieldStatus.Unknown
            && snapshot.Cards[0].ResolvedCost.Status == LiveCardFieldStatus.Unknown
            && snapshot.Cards[0].FixedEnergyCost.Status == LiveCardFieldStatus.NotObserved,
            "negative or unavailable raw fields are statuses, never clamped zero");
        fixture.Card.EnergyCost.Throw = true;
        Check(!fixture.Reader.Read("instance", "manifest").Available,
            "a native accessor failure produces no partial capture");
        fixture.Card.EnergyCost.Throw = false;
        fixture.Source.OnObserve = () => fixture.Source.Generation++;
        snapshot = fixture.Reader.Read("instance", "manifest");
        Check(!snapshot.Available && snapshot.UnavailableReason == "mixed_generation",
            "generation change during owner extraction produces no mixed snapshot");
    }
}
