#!/usr/bin/env bash
#
# One-time developer setup: toolchain components and the cargo tools the check
# scripts and CI expect.
#
#   ./scripts/setup.sh                 core tooling
#   ./scripts/setup.sh --with-nightly  also install nightly + cargo-fuzz

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(dirname "$script_dir")"
cd "$repo_root"

with_nightly=0
[[ "${1:-}" == "--with-nightly" ]] && with_nightly=1

command -v rustup >/dev/null 2>&1 || {
  echo "rustup not found. Install from https://rustup.rs and re-run." >&2
  exit 1
}

echo "Installing the pinned toolchain..."
rustup show active-toolchain   # rust-toolchain.toml pins it; this materialises it

install_tool() {
  local bin="$1" crate="$2" why="$3"
  if command -v "$bin" >/dev/null 2>&1; then
    echo "have  $bin"
  else
    echo "install  $bin  ($why)"
    cargo install "$crate" --locked
  fi
}

install_tool cargo-nextest  cargo-nextest  'test runner'
install_tool cargo-deny     cargo-deny     'licence and advisory policy'
install_tool cargo-audit    cargo-audit    'RUSTSEC advisories'
install_tool cargo-llvm-cov cargo-llvm-cov 'coverage gate on domain + simulation'

if (( with_nightly )); then
  # cargo-fuzz requires nightly; only the fuzz job and Phase 5 need it.
  rustup toolchain install nightly --profile minimal
  install_tool cargo-fuzz cargo-fuzz 'fuzz targets'
fi

[[ -f .env ]] || { cp .env.example .env; echo "created  .env from .env.example"; }

echo $'\nSetup complete. Next: ./scripts/check.sh'
