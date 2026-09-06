# ADR 0027: Native Crystal Sphere controls and combat settlement

Status: implemented; native Linux event exit verified. See
[live evidence](../evidence/crystal-sphere-linux-20260906.md).

The Crystal Sphere event opens a custom native screen while its parent event task remains
pending. The ordinary event-choice boundary previously waited for card selection or completion,
so it could not report the custom screen as an actionable boundary.

The addon now projects clickable unrevealed cells from the unique visible native screen. Each
legal choice identifies its public grid coordinates and explicitly uses the currently selected
native tool. It invokes the native cell control and awaits removal of that cell's fog before
settlement. The parent event remains owned until its native task completes. Parent selection
excludes the child cell operations, preventing ambiguous ownership during reconciliation.

This adapter does not read hidden items, mutate the random generator, select a different tool,
or force minigame completion. Public native APIs and visible control state are its only inputs.
Tool selection is not exposed by this change. A missing or ambiguous screen, modal, or excessive
cell count produces no admitted cell actions.

Leaving Crystal Sphere loot returns to the custom screen rather than directly to the map.
That reward operation settles only after the original sphere is again on top and its native
Proceed control is clickable. The separate `crystal_sphere:proceed` action invokes that control
and verifies native event choices take over or the map becomes visible and travelable. The native
screen can remain alive behind the map, so node removal is not an exit requirement. This final screen exit
does not depend on the earlier parent event task remaining incomplete.

Separately, a combat action may complete while the host is between combat and its reward screen.
A recovery observation during that transition cannot establish a settled gameplay boundary.
The combat completion probe therefore retains the operation until an authoritative actionable
observation or terminal outcome is available. A blocked combat frame can still precede the recovery
gap, and the host may briefly retain playable cards after every enemy reaches zero HP. Neither
frame is an admitted settlement boundary. A nonterminal boundary must expose current legal actions.
An end-turn action can settle at the next player turn or after combat has ended. The caller continues reconciling the same
operation identity. Crystal Sphere cells are admitted only while their screen is the top overlay;
its later loot overlay takes precedence over the still-visible grid.

Windows and Linux host compilation are compatibility evidence only. Live event settlement and
replay evidence must be recorded separately before claiming runtime verification.
