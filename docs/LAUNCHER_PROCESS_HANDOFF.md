# Bounded owned-process handoff

The Windows bridge is a session guardian. The shell opens a duplex pipe, sends one bounded
credential line, then reads exactly three receipt lines with an absolute startup deadline and
a 128-character per-line bound. It does not wait for stdout EOF. Missing or malformed receipts
trigger cancellation and report cleanup uncertainty, never an invented game PID or success.

The guardian supplies an unnamed, non-inheritable kill-on-close Job through
`PROC_THREAD_ATTRIBUTE_JOB_LIST` in the same extended `CreateProcessW` call as the NUL-only
handle list. There is no create-then-assign gap. Unsupported configurations fail closed;
Windows 10 / Server 2016 or newer is required. See Microsoft's
[creation attributes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute).

The Job handle is not inherited by the child. Closing its last handle terminates associated
processes. Ordinary descendants inherit membership; arbitrary externally launched services are
not an owned tree or a privilege sandbox. No breakaway limits are enabled. See Microsoft's
[Job lifetime rules](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

Microsoft's [WinBase.h](https://raw.githubusercontent.com/microsoft/win32metadata/main/generation/WinSDK/RecompiledIdlHeaders/um/WinBase.h)
defines input flag `0x20000`, handle-list number `2` and Job-list number `13`: the interop values
are therefore `0x20002` and `0x2000d`, not guessed constants. Attribute storage and handles remain
alive through creation. Only NUL handles are in the child's inherited-handle list.

The required `--lease-seconds` is 1–3600, capped by remaining authorization. One hour is an
additional session limit, including keep-alive. A retained timer starts before credential input;
EOF/cancellation has a separate background reader after that input. Both exit the guardian even
if receipt publication stalls, closing the Job. The timer uses the elapsed-time clock described
by [System.Threading.Timer](https://learn.microsoft.com/en-us/dotnet/api/system.threading.timer?view=net-9.0),
not repeated wall-clock comparisons; scheduling is not a real-time guarantee.

The shell closes the control pipe and stops only its recorded guardian group. With a complete
receipt it can additionally verify PID, creation time and executable before cleanup. Without a
receipt it reports uncertainty across WSL. The harness does not inherit the guardian pipe.
Prebuilt bridges must be rebuilt for this persistent protocol. Windows synthetic tests cover
credential/receipt stalls, EOF, explicit cancellation, guardian death, lease and descendant cleanup;
Linux shell tests cover stalled, partial and oversized receipts. No exact WSL/game run is claimed.

## Build output identity

Packaging obtains Cargo's effective target directory from `cargo metadata`; provider builds use
the executable path in Cargo's `compiler-artifact` JSON. Both require `jq` and honor Cargo output
configuration, including `CARGO_TARGET_DIR`. Tests retain stale default binaries while selecting
new bytes from an alternate output directory; no assumed `target/debug` fallback is used.
