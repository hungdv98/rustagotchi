# Security Policy

## Reporting a vulnerability

Report privately via GitHub's **Report a vulnerability** button on the Security
tab. Do not open a public issue.

Please include the affected version, reproduction steps, and what an attacker
gains. We aim to acknowledge within 3 working days and to ship a fix or a
mitigation plan within 30 days.

## Scope

Rustagotchi is a single-player desktop game with no network features and no
account system, so the realistic threat model is narrow. In scope:

- **Save file parsing.** Save files are attacker-controlled input in practice —
  players trade them. A malformed or hostile save must produce an error, never
  memory unsafety, an unbounded allocation, or arbitrary code execution.
  Continuously fuzzed by `fuzz/fuzz_targets/load_save.rs`.
- **Content pack parsing.** Content packs are data and will be moddable. Same
  guarantee. Fuzzed by `fuzz/fuzz_targets/parse_content.rs`.
- **Path handling.** Save and asset paths must not escape their configured roots.
- **Dependency vulnerabilities.** `cargo audit` and `cargo deny` run on every
  pull request and nightly.

Out of scope: cheating in single-player (editing your own save is not an attack),
and denial of service caused by a user deliberately supplying a pathological save
to their own machine.

## Hardening in place

- `unsafe_code = "deny"` across the workspace.
- Save payloads are checksummed; corruption is detected and a rotating backup is
  restored rather than loading partial state.
- Decompression is bounded — a save that expands beyond a configured limit is
  rejected rather than exhausting memory.
- Crash reports capture the panic payload, backtrace, build version and current
  tick. They never include save contents or player-authored text.
