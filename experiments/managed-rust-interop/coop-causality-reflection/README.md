# Native co-op causal-provenance metadata probe

This build-only probe reads the exact supplied managed assembly's PE metadata through `PEReader`
and `MetadataReader`. It does **not** load that assembly, resolve its dependencies, instantiate a
type, run a static constructor, or invoke a method. It is a prerequisite for any versioned
client-actuation contract: matching an actor/action/generation tuple is not causal provenance, so
no gateway or adapter may infer settlement from it.

Review warning: this tool is intentionally metadata-only. Do not replace it with `Assembly.Load*`,
`GetTypes`, `MetadataLoadContext`, or `AssemblyLoadContext`; those approaches can resolve or
execute code from a reserved/private host assembly and invalidate the inspection safety boundary.
The companion synthetic fixture test guards those APIs and proves that its sentinel static
constructor is not executed.

This source-only probe requires independent review before it may inspect any reserved host
assembly. After that review and only with root-reserved, read-only access to the exact host data
directory containing `sts2.dll`, run:

```text
dotnet run --project experiments/managed-rust-interop/coop-causality-reflection/CoopCausalityReflection.csproj -- \
  /path/to/data_sts2_linux_x86_64
```

The one-line JSON report lists metadata candidates for (1) a native action queue ID and (2)
first-party message-handler registration, including private, static, generic, overloaded, and
inherited-interface members. `causal_provenance` always stays `unproven`: metadata cannot
establish authenticated client-to-host delivery, preservation of a unique dispatch ID, or a
host-issued witness returned to the originating peer.

The deterministic source-only check is:

```text
dotnet run --project experiments/managed-rust-interop/coop-causality-reflection-tests/CoopCausalityReflectionTests.csproj --configuration Release
```

A versioned wire addition is authorized only after a reserved exact-host two-peer trace shows one
original operation identity reaching a unique native dispatch/queue ID or supported first-party
message carrier, then a host-issued witness reaches that same authenticated peer link. Otherwise
client mutations remain rejected or unknown; no fingerprint-correlation fallback is permitted.
