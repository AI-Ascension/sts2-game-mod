# Release recovery survivor/loss report

Recorded 2026-09-07T01:36Z after the WSL/lighthouse reboot. This report is
based on the persistent Git databases; it does not treat old process IDs,
temporary paths, or prior summaries as surviving artifacts.

## Surviving release source

The persistent `sts2-game-mod` Git database still contains the release branch
ref and every committed release object:

| commit | parent | subject |
| --- | --- | --- |
| `adb35ea595f3dd2a7c7c79d0e459d1a90f743930` | `c41064a7e76aec4ce0b862a9a1cb160236b16d9d` | release: add deterministic source bundles |
| `e6e8e2665cbfd4ba393f036e586ed8cc78a53c1c` | `adb35ea595f3dd2a7c7c79d0e459d1a90f743930` | release: normalize source bundle modes |
| `9b087a2fade6d1c014a4ee9ed536f52e7144e37f` | `e6e8e2665cbfd4ba393f036e586ed8cc78a53c1c` | release: document platform-specific Workshop boundary |
| `ea5c762a7eb04084535dd539ec618f68e170809d` | `9b087a2fade6d1c014a4ee9ed536f52e7144e37f` | release: add Windows and Linux runtime packaging |
| `0c27810d10fc39bdce10f982f3272c7c0e7d545e` | `ea5c762a7eb04084535dd539ec618f68e170809d` | release: harden runtime package lifecycle |
| `0fae297c1d1659a688e2618330a2700b3a4f0b42` | `0c27810d10fc39bdce10f982f3272c7c0e7d545e` | release: close runtime package lifecycle gates |

`refs/heads/codex/release-package-20260906` points at
`0fae297c1d1659a688e2618330a2700b3a4f0b42`; its parent chain reaches the
surviving `origin/main` `c41064a7e76aec4ce0b862a9a1cb160236b16d9d`.
The persistent recovery worktree is detached at that exact commit. The
committed tree contains the source bundle, platform runtime packaging, package
validation, installer/rollback, and synthetic lifecycle test tooling listed by
`git diff --name-status c41064a..0fae297`.

The persistent aggregate `live-v3-llm-20260905` also survives as a collection
of nested repositories. Release-relevant exact
heads observed there are: `mod` `56a3ca132b47d44bc90814c2bbb0a93486db562a`,
`sts2-game-core` `87e0f3d9355c0827e989d9fbc31804440852519b`, `sts2-gateway`
`d629156e4a5612f18a853487b754b29787de93f6`, `sts2-mcp-server`
`de3e256f7d47f669b9b9416b664ae581c5a43465`, `sts2-protocol`
`9d6d87f66737dcc01ac7577b5fe8b50b11b402aa`, `harness`
`e7934ec2ad59e62b9f45f6207ac9dda84ff92b7a`, `observability`
`013cd2f604ee14f48718d2a514b713e117277aca`, and `site`
`19917719dbe1b8ee9cd61fac89f556f93cb3a9e3`. These are preserved evidence
heads, not substitutes for the release branch.

## Lost or unverified temporary material

The reboot-scoped recovery worktrees, Windows/Linux candidate and real package
directories, temporary Steam/API probes, and temporary source-SDK checkout no
longer exist.
The final self-contained UGC helper (recorded SHA-256
`614c0ced0b0bdac154b3610a03b57c056d9a589222e76590edba9c1e9eeacc76`), the
temporary Workshop verify/upload/update/rollback/download/subscribe scripts,
their journals/flags, ABI proof, package outputs, host build outputs, and
process logs are therefore not surviving files. Their recorded digests are
historical until rebuilt and rechecked. The versioned lifecycle source and
synthetic tests have since been reconstructed under
`tools/workshop/lifecycle/` in this recovery worktree.

The committed `0fae297` tree does retain `tools/release/test-runtime-lifecycle.sh`
and the package/installer gates, but it does not yet retain the reconstructed
UGC operator or native callback proof. No public Workshop mutation is claimed
in this recovery report; the prior Windows/Linux Steam initialization checks
are historical read-only evidence and require fresh artifact identity checks
before publication.

## Fresh reconstruction checks

At `2026-09-07T02:33:58Z`, the recovered source passed the following local
checks in this worktree:

- `bash tools/workshop/test-package-item.sh`;
- `bash tools/release/test-runtime-lifecycle.sh`;
- `bash tools/release/test-source-bundle.sh` against committed `HEAD`;
- `bash tools/workshop/lifecycle/test-workshop-lifecycle.sh`;
- `bash tools/workshop/lifecycle/ugc-create-item/test-ugc-create-item.sh`, including
  Linux-only platform refusal, synthetic native dispatch, success, transport failure,
  timeout, journal collision, hash mismatch, and missing environment;
- `<pinned .NET 9 executable> build` for the operator, UGC helper, managed source-only
  probe, and Workshop validation probe, all with zero warnings and errors;
- `<pinned .NET 9 executable> run --project experiments/managed-rust-interop/workshop/WorkshopValidationProbe.csproj --configuration Release --no-build`;
- `bash tools/workshop/lifecycle/abi-proof/verify-sdk-abi.sh <temporary source SDK checkout>`,
  which passed commit `b8cfb12c0e083a2ef5b2f9f9b50f3902fa034474`, callback `3403`, size
  `16`, and offsets `(0,4,12)`;
- `cargo fmt --all --check`, locked/offline workspace build, warnings-denied Clippy,
  workspace tests, and strict `repo-policy`, all passed using a separate Cargo target
  directory.

The source-bundle test is a final-commit gate: rerun it whenever the reconstructed lifecycle files
or their surrounding release source changes so the archive includes the reviewed files.

## Current release blockers

No approved Windows or Linux native/managed candidate payload is present in this recovery lane,
and there is no assigned first-party Workshop item ID, publisher entitlement, or operator Steam
API/account input recorded here. The external SDK checkout used for the ABI proof is an
operator-supplied temporary input and is not part of the repository. Real Steam create,
upload, update, subscription/download, package discovery, loader startup, and host-runtime
evidence remain unverified. Root owns those host and Steam actions; the next step is to
supply disposable exact artifact inputs, run the guarded operations, and preserve their
fresh hashes/journals before any publication claim.

## Recovery boundary

All reconstruction belongs under the persistent recovery worktree. The dirty
shared checkout and surviving nested worktrees remain untouched. Native host,
GPU, Steam, public Workshop, deployment, and release-publication mutations
remain root-controlled; this lane will provide versioned source, fake tests,
and exact artifact inputs for root review.
