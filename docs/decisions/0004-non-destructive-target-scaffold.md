# ADR 0004: Non-destructive target scaffold

- Status: Accepted for foundation preparation
- Date: 2026-09-02

## Context

The aggregate plan separates the game core, game mod, gateway, MCP server, harness, and protocol
target. The mod tree already contains responsibility directories and a managed/native experiment.
Foundation work must not imply that an unimplemented product exists.

## Decision

Keep the target uninitialized as Git history and add repository governance, policy tooling,
toolchain metadata, docs, workflows, and decisions. The root Cargo workspace contains only the
target-local policy tool and the pre-existing experiment native crate. Do not add product crates,
empty packages, product schemas, game files, or generated output.

Preserve the experiment in place, including its existing managed projects and local generated
output. Generated output is not source and is not included in a package.

## Consequences

Foundation commands can validate policy and the existing source-level native seam without requiring
the product workspace or proprietary host files. The next owner must initialize real modules only
after recording their responsibility, consumer, contract, and test seam.
