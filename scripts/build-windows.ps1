#Requires -Version 5.1
<#
.SYNOPSIS
Builds the nose release binary natively on Windows.

.DESCRIPTION
nose is pure safe Rust with no platform-specific code; the only native piece
is the tree-sitter grammars' C sources, which the `cc` crate compiles with
MSVC. This script checks the two prerequisites (a Rust toolchain and the MSVC
Build Tools), runs the release build, and smoke-tests the produced binary —
including one real C# analysis so a broken grammar compilation cannot pass
silently. No CI, no cross-compilation: run it on the Windows machine that
needs the binary.

Prerequisites (one-time):
  winget install Rustlang.Rustup
  winget install Microsoft.VisualStudio.2022.BuildTools
    (select the "Desktop development with C++" workload, or pass
     --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended")

The pinned toolchain in rust-toolchain.toml is installed automatically by
rustup on the first cargo invocation.

.PARAMETER DebugBuild
Build the debug profile instead of release.

.PARAMETER SkipSmoke
Skip the post-build smoke test.

.EXAMPLE
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
#>
[CmdletBinding()]
param(
    [switch]$DebugBuild,
    [switch]$SkipSmoke
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    # --- prerequisites -----------------------------------------------------
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error @"
cargo not found. Install the Rust toolchain first:
  winget install Rustlang.Rustup
then reopen the terminal so PATH picks it up.
"@
    }

    # The MSVC linker is required by the tree-sitter grammars' C sources.
    # vswhere is installed alongside any Visual Studio/Build Tools >= 2017.
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $vcTools = & $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath 2>$null
        if (-not $vcTools) {
            Write-Warning ('No Visual Studio installation with the C++ toolchain was found. ' +
                'If the build fails at the linker step, install the Build Tools: ' +
                'winget install Microsoft.VisualStudio.2022.BuildTools (with the C++ workload).')
        }
    }
    else {
        Write-Warning ('vswhere.exe not found - cannot verify the MSVC Build Tools. ' +
            'If the build fails with a link.exe error, install: ' +
            'winget install Microsoft.VisualStudio.2022.BuildTools (with the C++ workload).')
    }

    # --- build -------------------------------------------------------------
    $profileName = if ($DebugBuild) { 'debug' } else { 'release' }
    $cargoArgs = @('build', '-p', 'nose-cli')
    if (-not $DebugBuild) { $cargoArgs += '--release' }

    Write-Host "building nose ($profileName) ..." -ForegroundColor Cyan
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "cargo build failed (exit $LASTEXITCODE)"
    }

    $binary = Join-Path $repoRoot "target\$profileName\nose.exe"
    if (-not (Test-Path $binary)) {
        Write-Error "expected binary not found at $binary"
    }

    # --- smoke test --------------------------------------------------------
    if (-not $SkipSmoke) {
        Write-Host 'smoke test ...' -ForegroundColor Cyan
        & $binary --version
        if ($LASTEXITCODE -ne 0) { Write-Error 'nose --version failed' }

        # One real analysis proves the tree-sitter grammars compiled and parse:
        # two trivially identical C# methods must yield exactly one family.
        $smokeDir = Join-Path ([System.IO.Path]::GetTempPath()) "nose-smoke-$PID"
        New-Item -ItemType Directory -Force -Path $smokeDir | Out-Null
        try {
            @'
class Smoke {
    static int A(int[] xs) { var s = 0; foreach (var x in xs) { s += x * 2 + 1; } return s; }
    static int B(int[] ys) { var t = 0; foreach (var y in ys) { t += y * 2 + 1; } return t; }
}
'@ | Set-Content -Path (Join-Path $smokeDir 'smoke.cs') -Encoding utf8
            # `all` includes families held below the default surface — this
            # fixture is deliberately tiny.
            $report = & $binary query $smokeDir --min-size 1 all --format json | ConvertFrom-Json
            if ($LASTEXITCODE -ne 0) { Write-Error 'nose query smoke run failed' }
            if (-not $report.families -or $report.families.Count -lt 1) {
                Write-Error 'smoke analysis found no duplicate family - grammar or pipeline problem'
            }
            Write-Host 'smoke test passed: C# analysis produced the expected family' -ForegroundColor Green
        }
        finally {
            Remove-Item -Recurse -Force $smokeDir -ErrorAction SilentlyContinue
        }
    }

    $size = [math]::Round((Get-Item $binary).Length / 1MB, 1)
    Write-Host "done: $binary ($size MB)" -ForegroundColor Green
}
finally {
    Pop-Location
}
