# PowerShell local-ci script for Chronos (Windows)
# Mirrors the behavior of local-ci.sh

$ErrorActionPreference = "Continue"

$Esc = [char]27
$Green = "$Esc[0;32m"
$Red = "$Esc[0;31m"
$Yellow = "$Esc[1;33m"
$NC = "$Esc[0m"

$Pass = 0
$Fail = 0

function Check-Command {
    param(
        [string]$Name,
        [scriptblock]$Script
    )
    Write-Host "${Yellow}=== [$Name] ===${NC}"
    
    # Reset LASTEXITCODE to ensure we don't carry over a previous status
    $global:LASTEXITCODE = 0
    
    # Run the script block
    & $Script
    $success = $?
    if ($LASTEXITCODE -ne 0) {
        $success = $false
    }
    
    if ($success) {
        Write-Host "${Green}OK: $Name passed${NC}`n"
        $global:Pass++
    } else {
        Write-Host "${Red}FAIL: $Name failed${NC}`n"
        $global:Fail++
    }
}

Write-Host "${Yellow}======================================${NC}"
Write-Host "${Yellow}  Local CI - Chronos (Windows)${NC}"
Write-Host "${Yellow}  $(Get-Date)${NC}"
Write-Host "${Yellow}======================================${NC}`n"

# ── Prechecks ─────────────────────────────────────────────
Check-Command "cargo fmt" { cargo fmt --all --check }

# ── Quality & Tests ───────────────────────────────────────
Check-Command "cargo clippy" { cargo clippy --workspace --all-targets -- -D warnings }
Check-Command "cargo test" { cargo test --workspace --all-features }

# ── Build ─────────────────────────────────────────────────
Check-Command "cargo build --release" {
    Stop-Process -Name "chronos" -ErrorAction SilentlyContinue
    cargo build --release
}

# ── Summary ───────────────────────────────────────────────
Write-Host "${Yellow}======================================${NC}"
if ($Fail -eq 0) {
    Write-Host "${Green}OK: ALL LOCAL VERIFICATIONS SUCCESSFUL! ($Pass passed)${NC}"
    Exit 0
} else {
    Write-Host "${Red}FAIL: VERIFICATIONS FAILED! ($Fail failed, $Pass passed)${NC}"
    Exit 1
}
Write-Host "${Yellow}======================================${NC}"
