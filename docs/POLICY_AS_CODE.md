# Policy as code

## Purpose

Written guidance is advisory. The target-local Rust repo-policy tool checks objective foundation
rules from policy.toml and is the local/CI entrypoint:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

The command is read-only with respect to the repository. Strict mode treats preferred-size
warnings as failures. It must not hide a missing tool, failed command, or unavailable host test.

## Enforced rule families

| Rule | Check |
| --- | --- |
| CFG001 | policy configuration exists and declares the supported version |
| DOC001 | required governance, docs, workflow, and tool files exist |
| DOC002 | local Markdown links resolve |
| DOC003 | normative ADR identifiers are unique and match numbered filenames; old-path redirects remain thin |
| SIZE001 | handwritten Rust, C#, workflow, and Markdown budgets |
| EXC001 | exemptions are exact existing paths with reasons |
| WF001-WF005 | workflow permissions, trust, failure visibility, and immutable actions |
| RUST001 | workspace metadata, lockfile, toolchain, and inherited lint policy |
| LANG001 | Python source and package metadata are prohibited |
| LIC001-LIC003 | MIT declarations and source SPDX headers |

The checker deliberately does not claim to prove game behavior, host compatibility, HTTP route
semantics, ABI compatibility with a real library, security enforcement, or package safety. The
initialized source seams have deterministic fake tests, but real host evidence remains separate.

## Configuration and changes

policy.toml lists required paths, ignored generated directories, size budgets, and exact
exemptions. An exemption must name a real generated, vendored, or reviewed static file and explain
its durable provenance. Copied implementation source is never eligible.

Changing policy is a process change. Explain the rule, enforcement effect, migration, reason for
any exemption, and exact local results. Refactor oversized handwritten files before weakening a
threshold.

The [decision registry](decisions/README.md) records the current numbering. DOC003 rejects reused
four-digit heading identifiers, filename/heading mismatches, and malformed decision headings.
Thin `# Moved:` redirects preserve old links without counting as duplicate normative decisions.
This repairs merged-branch identifier collisions without changing the decisions themselves; no
policy exemptions or weakened checks are needed. It does not prove agreement with decision content.

## CI relationship

policy.yml runs the same tests and strict command with contents read permission. ci.yml runs the
Rust gates and the native interop probe. The managed runtime-addon build remains outside ordinary
CI because it needs operator-supplied proprietary `sts2.dll` and `GodotSharp.dll` references.
Workflows use immutable action commits, explicit timeouts, bounded commands, no secrets, and no
privileged pull-request event.
