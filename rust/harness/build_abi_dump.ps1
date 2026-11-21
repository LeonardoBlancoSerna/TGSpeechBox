# Rebuilds rust/harness/abi_reference.txt from the C++ headers (MSVC via
# VS Build Tools 2022). Run after changing src/frame.h, src/sample.h or
# src/voicingTone.h so the Rust ABI tests track the C reference.
#
#   powershell -ExecutionPolicy Bypass -File rust\harness\build_abi_dump.ps1

$ErrorActionPreference = 'Stop'

$harness = $PSScriptRoot
$root = Split-Path -Parent (Split-Path -Parent $harness)

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere)) { throw "vswhere.exe not found" }
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw "No Visual C++ toolset found (VS Build Tools 2022)" }

$devcmd = Join-Path $vs 'Common7\Tools\VsDevCmd.bat'
$cpp = Join-Path $harness 'abi_dump.cpp'
$exe = Join-Path $harness 'abi_dump.exe'
$out = Join-Path $harness 'abi_reference.txt'

$cmd = "`"$devcmd`" -arch=amd64 -host_arch=amd64 >nul && " +
       "cl /nologo /std:c++17 /EHsc /O2 /I`"$root`" " +
       "/Fo`"$harness\abi_dump.obj`" /Fe`"$exe`" `"$cpp`" >nul && " +
       "`"$exe`" > `"$out`""
cmd /c $cmd
if ($LASTEXITCODE -ne 0) { throw "abi dump failed with exit code $LASTEXITCODE" }

Write-Host "Wrote $out ($((Get-Content $out).Count) lines)"
