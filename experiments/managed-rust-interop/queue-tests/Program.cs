// SPDX-License-Identifier: MIT

using System;
using System.Threading;
using System.Threading.Tasks;
using AiAscension.Sts2GameMod.Runtime;

var queue = new RuntimeDispatchQueue<int>(2);
var first = queue.Enqueue(1) ?? throw new InvalidOperationException();
var second = queue.Enqueue(2) ?? throw new InvalidOperationException();
Require(queue.Enqueue(3) == null, "capacity");
Require(queue.Wait(first, TimeSpan.Zero) ==
    (504, "{\"error_code\":\"main_thread_timeout_before_dispatch\"}"), "pending timeout");
Require(queue.Enqueue(3) != null, "timeout restores capacity");
int calls = 0;
queue.ProcessOne(value => { Require(value == 2, "FIFO and canceled work removal"); calls++; return (200, "second"); });
Require(queue.Wait(second, TimeSpan.Zero) == (200, "second"), "completed publication");
Require(calls == 1, "no canceled mutation");

var expired = new RuntimeDispatchQueue<int>(1);
var expiredWork = expired.Enqueue(1, TimeSpan.Zero) ?? throw new InvalidOperationException();
Require(expired.ProcessOne(_ => throw new InvalidOperationException("expired work executed")),
    "pump removes expired request even before waiter runs");
Require(expired.Wait(expiredWork, TimeSpan.Zero).Status == 504, "expired admission deadline");

var race = new RuntimeDispatchQueue<int>(1);
var pending = race.Enqueue(1) ?? throw new InvalidOperationException();
using var entered = new ManualResetEventSlim();
using var release = new ManualResetEventSlim();
Task execution = Task.Run(() => race.ProcessOne(_ =>
{
    entered.Set();
    if (!release.Wait(TimeSpan.FromSeconds(10))) throw new InvalidOperationException("barrier timeout");
    return (200, "late success");
}));
Require(entered.Wait(TimeSpan.FromSeconds(10)), "execution reached barrier");
var unknown = race.Wait(pending, TimeSpan.Zero);
Require(unknown == (504, "{\"error_code\":\"main_thread_outcome_unknown\"}"), "executing timeout stays unknown");
release.Set();
Require(execution.Wait(TimeSpan.FromSeconds(10)), "execution ended");
Require(race.Wait(pending, TimeSpan.Zero) == unknown, "late completion cannot rewrite response");
Require(!race.ProcessOne(_ => throw new InvalidOperationException()), "exactly one execution");
ModEntry.CheckCallback();
Console.WriteLine("Managed queue source-only regressions passed");

static void Require(bool value, string name)
{
    if (!value) throw new InvalidOperationException(name);
}
