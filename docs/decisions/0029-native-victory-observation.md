# ADR 0029: Current-run native victory observation

Status: implemented; native Windows forced-terminal verification confirmed.

The live source previously detected player death but had no native victory classification.
The candidate requires a living local single-player character, a non-abandoned run, a newly
positive host `WinTime`, and a visible native victory surface. This is either exactly one
visible game-over screen, or the host-declared `IsVictoryRoom` with its native event room
visible and no map or reward overlay. The host can enter its victory event while
`RunManager.IsInProgress` remains true. Practice fixtures
do not necessarily publish run history, so that record cannot be a required witness.

The source must have observed this same run object while it was active, before the terminal
screen appeared. It retains the host's prior win time and rejects an unchanged value, including
on repeated same-seed runs. Attaching only after victory does not establish this lineage and
remains unclassified. Terminal reads preserve the final player's public state.

These are observation checks only. They do not call the host end-run API, alter win time,
finish combat, or dismiss the terminal screen. The separately compiled forced-terminal fixture
uses host debug/command APIs and synthetic stale values. It is not a normally played campaign.

The Windows v0.107.1 fixture reached the native victory event after the last act's boss map
coordinate and native reward Proceed control. `WinTime` was 13 seconds, the player retained
80 HP, and the projection was Victory with disabled input and no legal mutations. Missing and
unchanged win markers and late attachment were rejected. See the
[native evidence](../evidence/native-victory-observation-20260906.md) for exact builds and limits.
