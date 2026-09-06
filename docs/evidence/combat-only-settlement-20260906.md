# Combat-only terminal settlement, 2026-09-06

The opt-in single-combat fixture projects Reward without further legal actions.
The campaign settlement guard had started requiring actionable reward controls, so
the final strike could visibly win combat while its receipt remained unknown.

The native reproduction reached Reward with 19 HP: 28 Astra decisions, 27 settled
receipts, and an unresolved `demo-op-28`. Reading the same operation again returned
`settlement_unproven`; it was not retried or counted as settled. The failed trajectory,
read-only reconciliation responses, profile, addon, and game log were preserved privately.

The fix treats Reward as terminal only when the source is explicitly in combat-only
mode. It retains the exact queued action, successful native completion, card/turn effect,
and generation witness requirements. Campaign mode still waits for usable controls.
Recovery and the intermediate state with all enemies dead remain insufficient.

## Native verification

A fresh Windows KVM game process with the normal addon and seed `AIASCENSIONREPLAY1`
completed 28 real OpenAI `gpt-6-astra` decisions and 28 settled actions. The harness
exited 0 with `combat_demo_complete`, Reward, and 19/80 HP. The final strike had its own
settled receipt. The visible RDP capture independently showed the reward screen and
19 HP, and the game log reported the 60-FPS cap.

This was a real single-combat run after cold VF recreation, not a full campaign or
a physical host reboot test. No OS mouse or keyboard automation drove the game.

| Private artifact | SHA-256 |
| --- | --- |
| Successful trajectory | `008b4bc60fee932ae435cdcc8458614c52d15bfafdc9000ca1c693ae8322a963` |
| Native Reward capture | `a9157d9aacf586f8774fbe34f7933b419bbb42c28d6f4e0dd71f56a28429780c` |
| Game log | `c8b0f0f77dd9e4bb87ea342e54305b9f021c61fb877375c08024a28ffed7a0fa` |
| Tested normal Windows addon | `32278a3cc8480f421a446a1cf51ddec6acf33b706b28a8fdc2658e751f216e59` |
| Linux host-dependent build, not installed by this test | `3718d1baf18410065bb7f52d39468bf160a10be906b4875004e7ebde8ea14e15` |

## Source verification

The managed gameplay probe passed, including the new distinction between a combat-only
reward and campaign/recovery/dead-enemy intermediate states. Rust formatting, Clippy
with warnings denied, all workspace tests, and strict policy passed. Both authorized
Windows and Linux host-dependent addon builds passed with zero warnings/errors and
terminal-fixture code disabled. Linux compilation is not new Linux native gameplay proof.
