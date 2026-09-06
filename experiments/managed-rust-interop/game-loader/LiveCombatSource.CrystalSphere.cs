// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Events.Custom.CrystalSphere;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private int _lastCrystalSphereCellCount = -1;
    private static NCrystalSphereScreen? CrystalSphereScreen()
    {
        if (NGame.Instance is not { } game) return null;
        NCrystalSphereScreen[] screens = Descendants(game).OfType<NCrystalSphereScreen>()
            .Where(screen => GodotObject.IsInstanceValid(screen) && screen.IsVisibleInTree()).ToArray();
        return screens.Length == 1 ? screens[0] : null;
    }

    // Coordinates and visible fog are public UI state. Never inspect a cell's item or the
    // minigame's hidden item collection, and never change its RNG, tool, or completion state.
    private static NCrystalSphereCell[] CrystalSphereCells(NCrystalSphereScreen screen) => Descendants(screen)
        .OfType<NCrystalSphereCell>().Where(cell => Clickable(cell) && cell.Entity.IsHidden)
        .OrderBy(cell => cell.Entity.Y).ThenBy(cell => cell.Entity.X).ToArray();

    private static string CrystalSphereId(NCrystalSphereCell cell) =>
        $"crystal_sphere:reveal_with_current_tool:{cell.Entity.X}:{cell.Entity.Y}";

    private RuntimeV3GameplayObservation ProjectCrystalSphere(RuntimeV3GameplayObservation observation,
        NCrystalSphereScreen screen)
    {
        NCrystalSphereCell[] cells = CrystalSphereCells(screen);
        if (_lastCrystalSphereCellCount != cells.Length)
        {
            _lastCrystalSphereCellCount = cells.Length;
            GD.Print($"[AI-ASCENSION LIVE] crystal sphere visible unrevealed cells={cells.Length}");
        }
        bool admitted = EventChoiceParent() != null && cells.Length is > 0 and <= RuntimeV3GameplayContract.MaxEntities
            && MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal == null;
        return Surface(observation, RuntimeV3GameplayState.Event,
            admitted ? cells.Select(CrystalSphereId).ToArray() : Array.Empty<string>(), admitted);
    }

    private RuntimeV3HostCompletion? CrystalSphereBoundary(RuntimeV3OperationKey operation,
        CampaignPending pending)
    {
        if (!ReferenceEquals(EventChoiceParent(), pending) || CrystalSphereScreen() == null) return null;
        RuntimeV3GameplayObservation after = Observe();
        if (after.State != RuntimeV3GameplayState.Event || !after.InputEnabled
            || after.Generation <= pending.Before.Generation) return null;
        var witness = new RuntimeV3TransitionWitness(operation, pending.Action, pending.Before.Generation,
            after.Generation, after.StateId, "event_crystal_sphere_requested");
        return new(after, witness, LegalActions(after));
    }

    private bool PrepareCrystalSphere(LegalActionReference action, out Func<Task> invoke,
        out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (CrystalSphereScreen() is not { } screen || !ReferenceEquals(RewardOverlay(), screen)
            || EventChoiceParent() is not { } parent) return false;
        NCrystalSphereCell? cell = CrystalSphereCells(screen)
            .SingleOrDefault(candidate => CrystalSphereId(candidate) == action.Value);
        if (cell == null) return false;
        var entity = cell.Entity;
        invoke = async () =>
        {
            cell.ForceClick();
            for (int frame = 0; frame < 600; frame++)
            {
                await WaitCampaignFrameAsync();
                RequireThread();
                if (!entity.IsHidden) return;
            }
            throw new InvalidOperationException("native crystal sphere cell did not reveal");
        };
        postcondition = () => !entity.IsHidden && (CrystalSphereScreen() == screen
            || parent.Work?.IsCompletedSuccessfully == true && parent.Postcondition());
        effect = "crystal_sphere_cell_revealed";
        return true;
    }
}
