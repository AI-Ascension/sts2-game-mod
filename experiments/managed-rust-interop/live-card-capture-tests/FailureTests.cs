// SPDX-License-Identifier: MIT
using System;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class Program
{
    private static void Failures()
    {
        var f = Setup();
        var snapshot = Read(f.Reader);
        Check(!f.Reader.Read("", "manifest").Available && Handles(f.Source) == 0,
            "invalid read identity clears source handles");
        Check(!Retained(f.Reader, snapshot).Available, "identity failure cannot reuse retained");
        snapshot = Read(f.Reader);
        Check(!f.Reader.ReadRetained("other", "manifest", snapshot.Epoch).Available
            && Handles(f.Source) == 0, "retained identity mismatch invalidates");
        snapshot = Read(f.Reader);
        f.Player.Deck.Cards.Add(f.Card);
        Check(!Read(f.Reader).Available && Handles(f.Source) == 0,
            "registry duplicate failure clears handles");
        f.Player.Deck.Cards.Clear();
        snapshot = Read(f.Reader);
        for (int index = 0; index < LiveCardSnapshotRegistry.MaxCards; index++)
            f.Player.Deck.Cards.Add(new FixedCard());
        Check(!Read(f.Reader).Available && Handles(f.Source) == 0,
            "capture overlimit clears handles");
        f.Player.Deck.Cards.Clear();
        snapshot = Read(f.Reader);
        f.Card.CurrentUpgradeLevel = -1;
        Check(!Read(f.Reader).Available && Handles(f.Source) == 0,
            "registry required field failure clears handles");
        f.Card.CurrentUpgradeLevel = 0;
        snapshot = Read(f.Reader);
        f.Thread.ThrowAfter = true;
        Check(!Read(f.Reader).Available && Handles(f.Source) == 0,
            "enqueue throws after capture clears source");
        f.Thread.ThrowAfter = false;
        Check(!Retained(f.Reader, snapshot).Available, "post-callback throw cannot retain");
        snapshot = Read(f.Reader);
        f.Thread.ThrowBefore = true;
        Check(!Read(f.Reader).Available, "enqueue refusal unavailable");
        f.Thread.ThrowBefore = false;
        Check(!Retained(f.Reader, snapshot).Available && Handles(f.Source) == 0,
            "enqueue refusal clears retained and subsequent owner call clears handles");
        snapshot = Read(f.Reader);
        bool wrongThread = Task.Run(() =>
        {
            try { f.Source.IsLiveCardSnapshotCurrent(snapshot); return false; }
            catch (InvalidOperationException) { return true; }
        }).GetAwaiter().GetResult();
        Check(wrongThread, "native handles cannot be read off owner thread");
        f.Reader.Invalidate();
        Check(Handles(f.Source) == 0 && !Retained(f.Reader, snapshot).Available,
            "explicit lifecycle invalidation clears handles");
    }

    private static void DeferredReads()
    {
        var f = Setup();
        var snapshot = Read(f.Reader);
        f.Thread.Deferred = true;
        Check(!Read(f.Reader).Available, "asynchronous capture refused");
        f.Thread.Drain();
        Check(Handles(f.Source) == 0, "deferred refused capture never restores handles");
        f.Thread.Deferred = false;
        Check(!Retained(f.Reader, snapshot).Available, "deferred callback cannot restore retained");
        snapshot = Read(f.Reader);
        f.Thread.Deferred = true;
        Check(!Retained(f.Reader, snapshot).Available, "asynchronous freshness refused");
        f.Thread.Drain();
        Check(Handles(f.Source) == 0, "deferred stale check cannot restore source");
        f.Thread.Deferred = false;
        snapshot = Read(f.Reader);
        f.Thread.Deferred = true;
        f.Reader.Invalidate();
        Check(!Read(f.Reader).Available, "deferred read after invalidation refused");
        f.Thread.Deferred = false;
        var newer = Read(f.Reader);
        f.Thread.Drain();
        Check(newer.Available && Retained(f.Reader, newer) == newer,
            "old delayed capture/invalidation cannot alter newer read");
        f.Thread.Deferred = true;
        f.Thread.ThrowAfter = true;
        Check(!Read(f.Reader).Available, "queued then throwing read refused");
        f.Thread.ThrowAfter = false;
        f.Thread.Drain();
        Check(Handles(f.Source) == 0, "callback after throwing enqueue never captures");
    }
}
