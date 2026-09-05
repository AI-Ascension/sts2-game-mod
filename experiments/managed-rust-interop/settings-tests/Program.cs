// SPDX-License-Identifier: MIT
using AiAscension.Sts2GameMod.Runtime;
using Godot;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

VideoPreferencesTests.Run();
var tree = (SceneTree)Engine.MainLoop!;
var save = SaveManager.Instance;
void Assert(bool value, string message)
{
    if (!value) throw new InvalidOperationException(message);
}
void Reset()
{
    save.CurrentProfileId = 1;
    save.IsProfileInitialized = true;
    save.CurrentRunSaveTask = null;
    save.Switches = save.Saves = 0;
    save.Progress = new();
    save.FailSave = false;
    RunManager.Instance.IsInProgress = false;
    GD.Messages.Clear();
}
void AssertUntouched(string reason) => Assert(
    save.Switches == 0 && save.Saves == 0 && save.Progress.Mutations == 0, reason);

Assert(AutoProfileUnlock.ScheduleManualUnlock(4).Contains("rejected"), "Invalid profile rejected");
AssertUntouched("Invalid profile did not mutate");
foreach (Task? pending in new Task?[] { new TaskCompletionSource().Task, Task.FromException(new InvalidOperationException()), Task.FromCanceled(new CancellationToken(true)) })
{
    Reset();
    save.CurrentRunSaveTask = pending;
    AutoProfileUnlock.ScheduleManualUnlock(2);
    tree.Tick();
    AssertUntouched("Pending/failed/canceled save prevented profile switching and writes");
}
Reset();
RunManager.Instance.IsInProgress = true;
AutoProfileUnlock.ScheduleManualUnlock(2);
tree.Tick();
AssertUntouched("Active run prevented switching and writes");

Reset();
AutoProfileUnlock.ScheduleManualUnlock(2);
tree.Tick();
Assert(save.Switches == 1 && save.Saves == 0, "Selected profile switch defers mutation");
RunManager.Instance.IsInProgress = true;
tree.Tick();
Assert(save.Saves == 0 && save.Progress.Mutations == 0, "Run starting between frames rejected");

Reset();
save.CurrentRunSaveTask = Task.CompletedTask;
save.Progress.MaxMultiplayerAscension = 12;
save.Progress.Stats.MaxAscension = 12;
Assert(AutoProfileUnlock.ScheduleManualUnlock(2).Contains("queued"), "Manual queue accepted");
Assert(AutoProfileUnlock.ScheduleManualUnlock(3).Contains("already"), "Concurrent request reported honestly");
tree.Tick();
tree.Tick();
Assert(save.CurrentProfileId == 2 && save.Saves == 1, "Only selected profile saved once");
Assert(save.Progress.MaxMultiplayerAscension == 12 && save.Progress.Stats.MaxAscension == 12, "Never reduce unlocked ascension");

Reset();
save.IsProfileInitialized = false;
AutoProfileUnlock.ScheduleManualUnlock(1);
for (int i = 0; i < 600; i++) tree.Tick();
AssertUntouched("Unready profile timeout made no writes");
save.IsProfileInitialized = true;
Assert(AutoProfileUnlock.ScheduleManualUnlock(1).Contains("queued"), "Timed-out attempt is retryable");
tree.Tick();
Assert(save.Saves == 1 && save.Progress.Stats.MaxAscension == 10, "Retry applies baseline unlock");

Reset();
save.FailSave = true;
AutoProfileUnlock.ScheduleManualUnlock(1);
tree.Tick();
Assert(save.Saves == 0 && !GD.Messages.Any(message => message.Contains("unlock applied")), "Save failure not reported as success");
save.FailSave = false;
AutoProfileUnlock.ScheduleManualUnlock(1);
tree.Tick();
Assert(save.Saves == 1, "Failed operation can be retried");
PersistenceTests.Run();
Console.WriteLine("Synthetic profile unlock and settings persistence guards passed; no host/game/save files used.");
