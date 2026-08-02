<#
.SYNOPSIS
    Architecture guard: asserts the dependency graph of the core crates has not
    changed without review.

.DESCRIPTION
    The dependency direction described in docs/init-plan.md is load-bearing:
    `domain` must never reach Bevy, SQLite, the filesystem or the OS. Comments
    do not enforce that; this does.

    For each guarded crate the normal (non-dev, non-build) dependency closure is
    compared against a committed snapshot under scripts/dep-allowlist/. Any new
    package -- direct or transitive -- fails the check until the snapshot is
    regenerated with -Update, which makes the addition show up in a diff.

    A denylist backstop additionally rejects a set of packages outright, so the
    failure message names the actual architectural violation rather than just
    reporting an unexpected package.

.PARAMETER Update
    Regenerate the snapshots instead of checking them. Review the resulting diff.

.EXAMPLE
    ./scripts/check-deps.ps1
    ./scripts/check-deps.ps1 -Update
#>
[CmdletBinding()]
param([switch]$Update)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$allowDir = Join-Path $PSScriptRoot 'dep-allowlist'

$guarded = @(
    'rustagotchi-domain'
    'rustagotchi-content'
    'rustagotchi-simulation'
    'rustagotchi-application'
)

# Packages that must never appear in the closure of any guarded crate, with the
# rule each one would violate.
$forbidden = @{
    'bevy'          = 'UI must not be reachable from the core (init-plan.md: dependency direction)'
    'bevy_ecs'      = 'UI must not be reachable from the core'
    'bevy_app'      = 'UI must not be reachable from the core'
    'winit'         = 'Windowing must not be reachable from the core'
    'wgpu'          = 'Rendering must not be reachable from the core'
    'egui'          = 'UI must not be reachable from the core'
    'rusqlite'      = 'Storage is an adapter behind application::ports::SaveRepository'
    'libsqlite3-sys'= 'Storage is an adapter behind application::ports::SaveRepository'
    'tokio'         = 'The core is synchronous and deterministic (ADR 0003)'
    'directories'   = 'Filesystem paths belong to crates/platform'
    'notify-rust'   = 'OS notifications belong to crates/platform'
    'fs4'           = 'File locking belongs to crates/platform'
    'reqwest'       = 'The core performs no network I/O'
}

function Get-DependencyClosure {
    param([string]$Package)

    $raw = & cargo tree --package $Package --edges normal --prefix none 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "cargo tree failed for '$Package':`n$($raw -join "`n")"
    }

    $raw |
        ForEach-Object { ($_ -split '\s+', 2)[0] } |
        Where-Object { $_ -and $_ -ne $Package } |
        Sort-Object -Unique
}

if (-not (Test-Path $allowDir)) {
    New-Item -ItemType Directory -Path $allowDir | Out-Null
}

$failed = $false

foreach ($pkg in $guarded) {
    $actual   = @(Get-DependencyClosure -Package $pkg)
    $snapshot = Join-Path $allowDir "$pkg.txt"

    # Denylist backstop -- reported first because the message is more useful.
    foreach ($dep in $actual) {
        if ($forbidden.ContainsKey($dep)) {
            Write-Host "FAIL  $pkg depends on '$dep'" -ForegroundColor Red
            Write-Host "      $($forbidden[$dep])" -ForegroundColor Red
            $failed = $true
        }
    }

    if ($Update) {
        # Written as BOM-less UTF-8 with LF endings so the bash counterpart --
        # which CI runs -- reads byte-identical content. Windows PowerShell's
        # `Set-Content -Encoding utf8` emits a BOM and CRLF, both of which show
        # up as spurious diffs on Linux.
        $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($snapshot, (($actual -join "`n") + "`n"), $utf8NoBom)
        Write-Host "updated  $pkg ($($actual.Count) packages)" -ForegroundColor Cyan
        continue
    }

    if (-not (Test-Path $snapshot)) {
        Write-Host "FAIL  no snapshot for $pkg. Run: ./scripts/check-deps.ps1 -Update" -ForegroundColor Red
        $failed = $true
        continue
    }

    $expected = @(Get-Content $snapshot | Where-Object { $_.Trim() })
    $added    = @($actual   | Where-Object { $expected -notcontains $_ })
    $removed  = @($expected | Where-Object { $actual   -notcontains $_ })

    if ($added.Count -or $removed.Count) {
        Write-Host "FAIL  $pkg dependency closure changed" -ForegroundColor Red
        foreach ($d in $added)   { Write-Host "      + $d" -ForegroundColor Red }
        foreach ($d in $removed) { Write-Host "      - $d" -ForegroundColor Yellow }
        Write-Host "      If intended: ./scripts/check-deps.ps1 -Update, then commit the diff." -ForegroundColor Yellow
        $failed = $true
    }
    else {
        Write-Host "ok    $pkg ($($actual.Count) packages)" -ForegroundColor Green
    }
}

if ($failed) { exit 1 }
if (-not $Update) { Write-Host "`nDependency direction holds." -ForegroundColor Green }
