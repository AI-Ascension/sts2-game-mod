# Runtime addon visible-overlay evidence

> Historical note: this report captures source revision `50eccb6d59a51ac9be561f23a67299017a55fca3`,
> before the visible overlay became opt-in. Its normal-launch screenshot remains valid for that
> revision; current source-derived behavior requires the standalone `--debug` argument. This PR
> does not claim a new host/runtime reproduction.

Date: 2026-09-02  
Status: `confirmed` for loader smoke and visible overlay; gameplay behavior remains `unverified`.  
Source revision: `50eccb6d59a51ac9be561f23a67299017a55fca3`

## Clean installed-mod test

The installed game's `mods/` directory was reduced to the three files owned by the addon:

~~~text
AIAscensionSTS2Poc.dll
AIAscensionSTS2Poc.json
ai_ascension_sts2_poc.dll
~~~

The 34 other files previously in that directory, including the MCP and interop-probe packages and
their backups, were moved—not deleted—to the sibling directory
`mods-disabled-20260902` so the cleanup remains reversible.

Package hashes installed for this test:

| File | SHA-256 |
| --- | --- |
| `AIAscensionSTS2Poc.dll` | `af3665328f5284666008982c3dd249da06cafd927869daf0c2a2f027a175c5ad` |
| `ai_ascension_sts2_poc.dll` | `30650a13b1748f0a27312f390013394de2f379adb880f2ab7acee3e1dbb8d8cd` |
| `AIAscensionSTS2Poc.json` | `a75717d4de14cf87d48b54b15fe45a3c58c231ef7395781b2e780d0a5e8c2985` |

## Live result

The real installed Windows executable was launched in a normal window with the addon enabled. The
game log recorded:

~~~text
[AI-ASCENSION STS2 POC] loaded managed entry point and Rust ABI; ABI=1; 19+23=42
[AI-ASCENSION STS2 POC] queued visible status overlay for the next safe frame
--- RUNNING MODDED! --- Loaded 1 mods (1 total)
[AI-ASCENSION STS2 POC] visible status overlay installed: AIAscensionSTS2PocStatus
~~~

The visible game window displayed this status banner over the STS2 main menu:

~~~text
AI-ASCENSION STS2 POC
WORKING | Rust ABI 1 | 19 + 23 = 42
~~~

An operator screenshot was captured at
`C:\Users\timot\AppData\Local\Temp\sts2-runtime-addon-window.png`; it is intentionally not
stored in the repository.

The test used the existing Steam-backed modded profile and caused the game's normal profile/save
sync writes during startup. No in-game action was performed. This confirms addon discovery,
managed initialization, native ABI validation, and a visible UI signal; it does not confirm HTTP,
host object mutation, action legality, effect settlement, or broader gameplay behavior.
