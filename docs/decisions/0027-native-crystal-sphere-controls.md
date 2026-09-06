# ADR 0027: Native Crystal Sphere controls and combat settlement

Status: source implemented; native event verification pending.

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
