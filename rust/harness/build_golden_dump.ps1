# Builds rust/harness/golden_dump.exe against the C++ reference libraries.
# Run build_x64 first (cmake -S . -B build-x64 -A x64 -DTGSB_BUILD_TESTS=ON
# and cmake --build build-x64 --config MinSizeRel).
#
#   powershell -ExecutionPolicy Bypass -File rust\harness\build_golden_dump.ps1

$ErrorActionPreference = 'Stop'

$harness = $PSScriptRoot
$root = Split-Path -Parent (Split-Path -Parent $harness)
$libDir = Join-Path $root 'build-x64\MinSizeRel'

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw "No Visual C++ toolset found (VS Build Tools 2022)" }

$devcmd = Join-Path $vs 'Common7\Tools\VsDevCmd.bat'
$cpp = Join-Path $harness 'golden_dump.cpp'
$exe = Join-Path $harness 'golden_dump.exe'
$speechPlayerLib = Join-Path $libDir 'speechPlayer.lib'
$frontendLib = Join-Path $libDir 'nvspFrontend.lib'

foreach ($f in @($speechPlayerLib, $frontendLib)) {
  if (-not (Test-Path $f)) { throw "Missing $f - build build-x64 (MinSizeRel) first" }
}

# /MT matches the reference build (CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded).
$cmd = "`"$devcmd`" -arch=amd64 -host_arch=amd64 >nul && " +
       "cl /nologo /std:c++17 /EHsc /O2 /MT /utf-8 " +
       "/I`"$root`" /I`"$root\src`" /I`"$root\src\frontend`" " +
       "/Fo`"$harness\golden_dump.obj`" /Fe`"$exe`" `"$cpp`" " +
       "`"$speechPlayerLib`" `"$frontendLib`" >nul"
cmd /c $cmd
if ($LASTEXITCODE -ne 0) { throw "golden_dump build failed with exit code $LASTEXITCODE" }

Write-Host "Built $exe"
