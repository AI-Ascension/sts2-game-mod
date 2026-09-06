# Native victory observation, 2026-09-06

Evidence scope: forced native Windows and Linux v0.107.1 terminal observation in isolated Train
practice fixture. This is not LLM control, a played boss fight, campaign victory, or replay.

The normal adapter now observes a living local single-player character, same active-run
object lineage, a changed positive host `WinTime`, and a visible native victory surface.
It projects Victory with disabled input and no legal mutations. It never changes host
win state or invokes an ending control. The test-only fixture is excluded from normal builds.

The fixture uses the public native APIs to select the last act and boss map coordinate,
force-kill test enemies, finish that combat, and activate the native reward Proceed control.
The resulting room was `EventRoom`, `IsVictoryRoom=true`, with no reward overlay. The host
reported `WinTime=13`, `IsInProgress=true`, `IsGameOver=false`, and player HP 80.
The observer reported Victory. A newly attached observer did not report Victory; the
original observer rejected its unchanged synthetic prior marker and an absent marker,
then reported Victory again when the fresh marker was restored.

Probe 13 confirmed these assertions before a readability-only fixture refactor:

| Artifact | SHA-256 |
| --- | --- |
| Probe addon | `678a2bffee4c6f0ce04ab5309cb86c7571504b38f36f8378059724a9eb6293a3` |
| Native game log | `6906a31544e9657070c9aa70fd03fd612b78eb10573347f04a8aa64711686df7` |

Logs and the replaced addon are retained in the isolated guest's operator backup. A fresh
read-only RDP capture showed the native victory event with the living player. No OS input
automation, provider call, private host implementation inspection, or save restoration
established this result. Earlier failed probes are retained separately and are not passes.

The final refactored fixture independently repeated all assertions with the same native
outcome. Its addon SHA-256 is
`8c417571c7e877c23a61ad84a4a44c4a534bd940026fec60075d4bdf2a911a57`;
the preserved native log is
`e34e170d213f386f117f0766d8ceaffa91ed4e8626b593ba6d63b8ea4672bb68`.

Normal Windows and Linux v0.107.1 addon builds succeeded with zero warnings and errors:

| Normal addon | SHA-256 |
| --- | --- |
| Windows | `982203153be150e627c4a26d865029bd5d90e572eb0970480da25a1eaf83405a` |
| Linux | `78420a13c278c99618340919396a667f22aa72b541365be32578b74480934f3e` |

Metadata inspection of these owned addons confirmed `TerminalProbe` exists only in the fixture
build and is absent from both normal builds. The Windows fixture was stopped after its log and
addon were preserved; the normal Windows addon and original practice launcher were restored,
hash-verified, and launched again.

The source-only managed build, Workshop validation probe, Rust workspace tests, Clippy with
warnings denied, formatting, and strict repository policy passed.
A model-played campaign win remains unverified.

## Linux native terminal verification

The same source-owned fixture was compiled against the exact Linux v0.107.1 assemblies
and launched separately after the prior replay controller had stopped. It selected the
last act and boss coordinate, force-ended that test fight through native APIs, and used
the native Proceed control. The host entered its visible victory event with `WinTime=19`,
80 HP, `IsInProgress=true`, and `IsGameOver=false`.

All assertions passed: Victory, disabled input, empty legal catalog, refusal of late
attachment, refusal of unchanged and absent markers, and restoration of the fresh result.
A read-only native capture independently showed the victory event and living player.
This is forced terminal-observation evidence, not model gameplay or replay.

| Linux fixture artifact | SHA-256 |
| --- | --- |
| Probe addon | `3f87bed72079bebbc0b0c0432abbbb04009fd85ed9094a92dc605973c3539bc4` |
| Preserved native log | `ad240d79072eeeb0af65dac26899db1238a52a5f52891a7e0e9d0d8b120282d0` |
| Native capture | `79fc1f1fbad19a183b189c682adb67d0208ae80637b309acfc014e9b46ce5ff0` |

The fixture addon, launcher and logs were backed up before its owned process was stopped.
The ordinary seeded launcher was retained separately; it does not enable the terminal fixture.
