#!/usr/bin/env bash
#
# Architecture guard: asserts the dependency graph of the core crates has not
# changed without review. POSIX counterpart of check-deps.ps1 -- see that file
# for the rationale. Both must stay behaviourally identical; CI runs this one.
#
#   ./scripts/check-deps.sh            check against the committed snapshots
#   ./scripts/check-deps.sh --update   regenerate the snapshots, then review the diff

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
allow_dir="$script_dir/dep-allowlist"
update=0
[[ "${1:-}" == "--update" ]] && update=1

guarded=(
  rustagotchi-domain
  rustagotchi-content
  rustagotchi-simulation
  rustagotchi-application
)

# Packages that must never appear in the closure of any guarded crate.
forbidden_names=(bevy bevy_ecs bevy_app winit wgpu egui rusqlite libsqlite3-sys tokio directories notify-rust fs4 reqwest)
forbidden_reason() {
  case "$1" in
    bevy|bevy_ecs|bevy_app|winit|wgpu|egui)
      echo "UI/windowing must not be reachable from the core (init-plan.md: dependency direction)" ;;
    rusqlite|libsqlite3-sys)
      echo "Storage is an adapter behind application::ports::SaveRepository" ;;
    tokio)
      echo "The core is synchronous and deterministic (ADR 0003)" ;;
    directories|notify-rust|fs4)
      echo "OS-dependent capabilities belong to crates/platform" ;;
    reqwest)
      echo "The core performs no network I/O" ;;
    *)
      echo "forbidden in the core" ;;
  esac
}

closure() {
  cargo tree --package "$1" --edges normal --prefix none \
    | awk '{print $1}' \
    | grep -v "^$1$" \
    | sort -u
}

mkdir -p "$allow_dir"
failed=0

for pkg in "${guarded[@]}"; do
  actual="$(closure "$pkg")"
  snapshot="$allow_dir/$pkg.txt"

  for bad in "${forbidden_names[@]}"; do
    if grep -qx "$bad" <<<"$actual"; then
      echo "FAIL  $pkg depends on '$bad'"
      echo "      $(forbidden_reason "$bad")"
      failed=1
    fi
  done

  if (( update )); then
    printf '%s\n' "$actual" > "$snapshot"
    echo "updated  $pkg ($(wc -l <<<"$actual" | tr -d ' ') packages)"
    continue
  fi

  if [[ ! -f "$snapshot" ]]; then
    echo "FAIL  no snapshot for $pkg. Run: ./scripts/check-deps.sh --update"
    failed=1
    continue
  fi

  if ! diff_out="$(diff <(grep -v '^\s*$' "$snapshot" | sort -u) <(printf '%s\n' "$actual") )"; then
    echo "FAIL  $pkg dependency closure changed"
    sed 's/^/      /' <<<"$diff_out"
    echo "      If intended: ./scripts/check-deps.sh --update, then commit the diff."
    failed=1
  else
    echo "ok    $pkg ($(wc -l <<<"$actual" | tr -d ' ') packages)"
  fi
done

(( failed )) && exit 1
(( update )) || echo $'\nDependency direction holds.'
exit 0
