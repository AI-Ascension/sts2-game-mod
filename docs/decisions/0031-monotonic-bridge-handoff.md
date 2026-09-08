# ADR 0031: Monotonic bridge handoff deadline

- Status: Proposed local implementation; owner review and remote adoption pending
- Date: 2026-09-08
- Scope: Linux/WSL shell guardian receipt deadline and synthetic timing assertions

## Evidence and decision

Reverification reproduced valid synthetic guardian rejection while Bash `SECONDS`
advanced by six seconds during less than one second of monotonic elapsed time.
An independent clock probe confirmed forward and backward wall-clock steps. A
deterministic test that advances `SECONDS` during receipt reads also failed before
this correction. The inspection test's elapsed-time assertion was similarly
vulnerable to clock adjustment.

Read whole elapsed seconds from Linux `/proc/uptime` for the existing absolute
handshake deadline and the two synthetic tests' duration assertions. This clock
includes suspend time and does not use calendar-clock adjustments. The existing
Linux/WSL launcher fails closed if that clock cannot be read or parsed; it does
not fall back to wall time or introduce another runtime language or dependency.
The kernel's [uptime implementation](https://github.com/torvalds/linux/blob/master/fs/proc/uptime.c)
uses the boot-time clock; its [timekeeping documentation](https://docs.kernel.org/core-api/timekeeping.html)
distinguishes that elapsed clock from adjustable calendar time.

Preserve the startup allowance, bounded three-line receipt, 128-character line
limit, credential channel, process ownership, cancellation, and one-hour guardian
lease. Authorization admission and expiry still compare their epoch deadline to
`EPOCHSECONDS`; elapsed timing does not extend or bypass that authorization.
No public API, ABI, receipt field, host access or runtime ownership changes.

## Verification and limits

Synthetic valid receipts must survive deterministic forward and backward changes
to Bash `SECONDS`. Stalled, partial and oversized receipts must still fail within
the existing bound. The inspection, installation refusal and expired-authorization
fixtures remain required. Tests use owned subprocesses and temporary fake files.

This corrects local elapsed-time handling; it does not establish Windows host,
game, suspend/resume, or live session compatibility. No real installation or
launch is authorized by this decision. Revert the implementation commit to roll
back source; no deployment, process, save or volume rollback is involved.
