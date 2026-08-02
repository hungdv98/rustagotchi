<#
.SYNOPSIS
    One-time developer setup: toolchain components and the cargo tools the
    check scripts and CI expect.
#>
[CmdletBinding()]
param([switch]$IncludeNightly)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Error "rustup not found. Install from https://rustup.rs and re-run."
}

# rust-toolchain.toml pins the exact version; this materialises it.
Write-Host "Installing the pinned toolchain..." -ForegroundColor Cyan
Push-Location $repoRoot
rustup show active-toolchain
Pop-Location

$tools = @(
    @{ Bin = 'cargo-nextest';  Crate = 'cargo-nextest';  Why = 'test runner' }
    @{ Bin = 'cargo-deny';     Crate = 'cargo-deny';     Why = 'licence and advisory policy' }
    @{ Bin = 'cargo-audit';    Crate = 'cargo-audit';    Why = 'RUSTSEC advisories' }
    @{ Bin = 'cargo-llvm-cov'; Crate = 'cargo-llvm-cov'; Why = 'coverage gate on domain + simulation' }
)

foreach ($t in $tools) {
    if (Get-Command $t.Bin -ErrorAction SilentlyContinue) {
        Write-Host "have  $($t.Bin)" -ForegroundColor DarkGray
    }
    else {
        Write-Host "install  $($t.Bin)  ($($t.Why))" -ForegroundColor Cyan
        cargo install $t.Crate --locked
    }
}

if ($IncludeNightly) {
    # cargo-fuzz requires nightly; only the fuzz job and Phase 5 need it.
    Write-Host "Installing nightly + cargo-fuzz..." -ForegroundColor Cyan
    rustup toolchain install nightly --profile minimal
    if (-not (Get-Command cargo-fuzz -ErrorAction SilentlyContinue)) {
        cargo install cargo-fuzz --locked
    }
}

if (-not (Test-Path (Join-Path $repoRoot '.env'))) {
    Copy-Item (Join-Path $repoRoot '.env.example') (Join-Path $repoRoot '.env')
    Write-Host "created  .env from .env.example" -ForegroundColor Green
}

Write-Host "`nSetup complete. Next: ./scripts/check.ps1" -ForegroundColor Green
