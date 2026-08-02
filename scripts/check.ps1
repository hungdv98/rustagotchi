<#
.SYNOPSIS
    The full local gate. Run this before opening a pull request; CI runs the
    same steps in the same order.

.PARAMETER Fast
    Skip the release build and the supply-chain checks. Use during iteration.

.PARAMETER Fix
    Apply `cargo fmt` and `cargo clippy --fix` instead of only reporting.
#>
[CmdletBinding()]
param(
    [switch]$Fast,
    [switch]$Fix
)

# Deliberately 'Continue': Windows PowerShell turns every stderr line from a
# native executable into an ErrorRecord, and under 'Stop' that aborts the script
# on cargo's ordinary progress output. Success is judged by $LASTEXITCODE.
$ErrorActionPreference = 'Continue'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot

$script:failures = @()

function Invoke-Step {
    param([string]$Name, [scriptblock]$Body, [switch]$Optional, [string]$MissingTool)

    if ($MissingTool -and -not (Get-Command $MissingTool -ErrorAction SilentlyContinue)) {
        Write-Host "SKIP  $Name (install with: cargo install $MissingTool)" -ForegroundColor DarkGray
        return
    }

    Write-Host "`n=== $Name ===" -ForegroundColor Cyan
    & $Body
    if ($LASTEXITCODE -ne 0) {
        if ($Optional) {
            Write-Host "WARN  $Name failed (non-blocking)" -ForegroundColor Yellow
        }
        else {
            Write-Host "FAIL  $Name" -ForegroundColor Red
            $script:failures += $Name
        }
    }
    else {
        Write-Host "ok    $Name" -ForegroundColor Green
    }
    $global:LASTEXITCODE = 0
}

if ($Fix) {
    Invoke-Step 'fmt (fix)'    { cargo fmt --all }
    Invoke-Step 'clippy (fix)' { cargo clippy --workspace --all-targets --all-features --fix --allow-dirty --allow-staged }
}

Invoke-Step 'fmt'    { cargo fmt --all -- --check }
Invoke-Step 'clippy' { cargo clippy --workspace --all-targets --all-features -- -D warnings }

# The architecture guard. Cheap, and the one most likely to catch a real
# mistake, so it runs before the test suite.
Invoke-Step 'deps'   { & (Join-Path $PSScriptRoot 'check-deps.ps1') }

if (Get-Command cargo-nextest -ErrorAction SilentlyContinue) {
    Invoke-Step 'test' { cargo nextest run --workspace --all-features }
    # nextest cannot run doctests; they need a separate pass.
    Invoke-Step 'doctest' { cargo test --workspace --all-features --doc }
}
else {
    Write-Host "`nNOTE  cargo-nextest not found, falling back to cargo test." -ForegroundColor DarkGray
    Write-Host "      Install with: cargo install cargo-nextest --locked" -ForegroundColor DarkGray
    Invoke-Step 'test' { cargo test --workspace --all-features }
}

if (-not $Fast) {
    Invoke-Step 'deny'  { cargo deny check }  -MissingTool 'cargo-deny'
    Invoke-Step 'audit' { cargo audit }       -MissingTool 'cargo-audit'
    Invoke-Step 'release build' { cargo build --workspace --release }

    # Enabled from Phase 2, when there is content to validate.
    if (Test-Path (Join-Path $repoRoot 'assets/manifest.ron')) {
        Invoke-Step 'validate-content' { cargo run -p simulation-cli -- validate-content }
    }
}

Pop-Location

if ($script:failures.Count) {
    Write-Host "`n$($script:failures.Count) step(s) failed: $($script:failures -join ', ')" -ForegroundColor Red
    exit 1
}

Write-Host "`nAll checks passed." -ForegroundColor Green
