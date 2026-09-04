// SPDX-License-Identifier: MIT

using AiAscension.Sts2GameMod.Runtime;
using Godot;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Runs;
using MegaCrit.Sts2.Core.Saves;

SeedReplayController.Initialize();
void Assert(bool condition, string message)
{
    if (!condition) throw new InvalidOperationException(message);
}
void Reset()
{
    RunManager.Instance = new();
    SaveManager.Instance = new();
    NGame.Instance = new();
    StandaloneProfileSettings.AllowRepeatingSeeds = true;
    GD.Errors.Clear();
}
void Queue()
{
    Assert(StandaloneProfileSettings.Replay!(), "eligible queue");
    Assert(!StandaloneProfileSettings.Replay!(), "duplicate queue");
}
void NoMutation(string scenario) => Assert(RunManager.Instance.Cleanups == 0
    && SaveManager.Instance.Deletes == 0 && NGame.Instance.Starts == 0, scenario);

Reset();
RunManager.Instance.Mode = null;
Assert(!StandaloneProfileSettings.Replay!(), "unknown mode with modifiers must reject");
foreach (GameMode mode in new[] { GameMode.Standard, GameMode.Daily })
{
    RunManager.Instance.Mode = mode;
    Assert(!StandaloneProfileSettings.Replay!(), "protected mode");
}
NoMutation("mode guards");
Reset();
Queue();
StandaloneProfileSettings.AllowRepeatingSeeds = false;
Engine.Tree.Pump();
NoMutation("opt-out before frame");
foreach (string scenario in new[] { "opt-out", "profile", "run", "replacement-save", "save-fault" })
{
    Reset();
    var save = new TaskCompletionSource();
    SaveManager.Instance.CurrentRunSaveTask = save.Task;
    Queue();
    Engine.Tree.Pump();
    NoMutation("save must drain");
    switch (scenario)
    {
        case "opt-out": StandaloneProfileSettings.AllowRepeatingSeeds = false; break;
        case "profile": SaveManager.Instance.CurrentProfileId = 2; break;
        case "run": RunManager.Instance.State = new(); break;
        case "replacement-save": SaveManager.Instance.CurrentRunSaveTask = new TaskCompletionSource().Task; break;
    }
    if (scenario == "save-fault") save.SetException(new InvalidOperationException("PRIVATE-PATH"));
    else save.SetResult();
    NoMutation(scenario);
    Assert(!GD.Errors.Any(error => error.Contains("PRIVATE-PATH", StringComparison.Ordinal)), "sanitized save error");
}
Reset();
Queue();
Engine.Tree.Pump();
Assert(RunManager.Instance.Cleanups == 1 && SaveManager.Instance.Deletes == 1
    && NGame.Instance.Starts == 1 && NGame.Instance.Seed == "SYNTHETIC", "exactly one same-seed restart");
Reset();
NGame.Instance.Fail = true;
Queue();
Engine.Tree.Pump();
Assert(GD.Errors.Any(error => error.Contains("after cleanup began", StringComparison.Ordinal)), "destructive failure status");
Assert(!GD.Errors.Any(error => error.Contains("PRIVATE-PATH", StringComparison.Ordinal)), "sanitized restart error");
Console.WriteLine("Replay source-linked admission, save-race, isolation and failure regressions passed.");
