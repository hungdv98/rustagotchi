#!/usr/bin/env bash
#
# The full gate. CI runs this; run it locally before opening a pull request.
#
#   ./scripts/check.sh          everything
#   ./scripts/check.sh --fast   skip the release build and supply-chain checks

set -uo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(dirname "$script_dir")"
cd "$repo_root"

fast=0
[[ "${1:-}" == "--fast" ]] && fast=1

failures=()

step() {
  local name="$1"; shift
  printf '\n=== %s ===\n' "$name"
  if "$@"; then
    echo "ok    $name"
  else
    echo "FAIL  $name"
    failures+=("$name")
  fi
}

optional_step() {
  local name="$1" tool="$2"; shift 2
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "SKIP  $name (install with: cargo install $tool --locked)"
    return
  fi
  step "$name" "$@"
}

step   'fmt'    cargo fmt --all -- --check
step   'clippy' cargo clippy --workspace --all-targets --all-features -- -D warnings

# The architecture guard. Cheap, and the one most likely to catch a real
# mistake, so it runs before the test suite.
step   'deps'   "$script_dir/check-deps.sh"

if command -v cargo-nextest >/dev/null 2>&1; then
  step 'test'    cargo nextest run --workspace --all-features
  # nextest cannot run doctests; they need a separate pass.
  step 'doctest' cargo test --workspace --all-features --doc
else
  echo $'\nNOTE  cargo-nextest not found, falling back to cargo test.'
  echo '      Install with: cargo install cargo-nextest --locked'
  step 'test' cargo test --workspace --all-features
fi

if (( ! fast )); then
  optional_step 'deny'  cargo-deny  cargo deny check
  optional_step 'audit' cargo-audit cargo audit
  step 'release build' cargo build --workspace --release

  # Enabled from Phase 2, when there is content to validate.
  if [[ -f assets/manifest.ron ]]; then
    step 'validate-content' cargo run -p simulation-cli -- validate-content
  fi
fi

if (( ${#failures[@]} )); then
  printf '\n%d step(s) failed: %s\n' "${#failures[@]}" "${failures[*]}"
  exit 1
fi

echo $'\nAll checks passed.'
