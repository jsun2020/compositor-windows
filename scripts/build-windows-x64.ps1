[CmdletBinding()]
param([switch]$SkipWasm, [switch]$NoSmoke)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $root
function Step([string]$m) { Write-Host "[build-windows-x64] $m" }
$buildInfoPath = Join-Path $root 'app/src/build-info.ts'
$originalBuildInfo = [IO.File]::ReadAllBytes($buildInfoPath)

try {
  # MSVC environment
  $vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
  if (-not (Test-Path $vswhere)) { throw 'vswhere.exe not found. Install Visual Studio 2022 Build Tools with the C++ workload.' }
  $installPath = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1
  if (-not $installPath) { throw 'VC.Tools.x86.x64 workload not found.' }
  $vsDevCmd = Join-Path $installPath 'Common7\Tools\VsDevCmd.bat'
  $env:VSCMD_SKIP_SENDTELEMETRY = '1'
  $envDump = & cmd.exe /d /s /c "`"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && set"
  foreach ($line in $envDump) { if ($line -match '^(.*?)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') } }
  $cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
  if (-not (($env:Path -split ';') -contains $cargoBin)) { $env:Path = "$cargoBin;$env:Path" }

  $version = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version
  Step "Writing build marker"
  $marker = & powershell -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\write-build-info.ps1') -Version $version
  $marker = ($marker | Select-Object -Last 1).Trim()
  Step "Marker: $marker"

  if (-not $SkipWasm) { Step 'Building wasm engine'; & pnpm wasm; if ($LASTEXITCODE -ne 0) { throw 'wasm build failed' } }
  Step 'Verifying wasm binary and glue exports'; & node (Join-Path $root 'scripts\verify-wasm-output.mjs'); if ($LASTEXITCODE -ne 0) { throw 'wasm output verification failed' }
  Step 'Building web app'; & pnpm build; if ($LASTEXITCODE -ne 0) { throw 'web build failed' }

  Step 'Verifying marker is bundled'
  $hits = Get-ChildItem (Join-Path $root 'app\dist\assets') -File | Where-Object { (Get-Content $_.FullName -Raw) -match [regex]::Escape($marker) }
  if (@($hits).Count -ne 1) { throw "Expected the marker in exactly one bundle file, found $(@($hits).Count)." }

  Step 'Building Tauri shell'
  $env:TAURI_ENV_TARGET_TRIPLE = 'x86_64-pc-windows-msvc'
  & pnpm tauri build --no-bundle --ci; if ($LASTEXITCODE -ne 0) { throw 'tauri build failed' }

  $exeCandidates = @(
    (Join-Path $root 'target\release\compositor-shell.exe'),
    (Join-Path $root 'target\release\Compositor.exe'),
    (Join-Path $root 'src-tauri\target\release\compositor-shell.exe'),
    (Join-Path $root 'src-tauri\target\release\Compositor.exe')
  )
  $exe = $exeCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $exe) { throw "Built exe not found. Tried: $($exeCandidates -join ', ')" }
  Step "Found exe: $exe"

  $stamp = ($marker -split '_')[-1]
  $outDir = Join-Path $root 'build-artifacts\windows-x64'
  $stage = Join-Path $outDir "Compositor-portable-$version-$stamp"
  $zip = "$stage.zip"
  if ((Test-Path -LiteralPath $stage) -or (Test-Path -LiteralPath $zip)) { throw 'Portable output already exists; preserve it and build at a new timestamp.' }
  New-Item -ItemType Directory -Path $stage | Out-Null
  Copy-Item $exe (Join-Path $stage 'Compositor.exe')
  Step 'Building pinned camera RAW decoder and including its license notices'
  & powershell -ExecutionPolicy Bypass -File (Join-Path $root 'scripts\build-raw-helper.ps1') -OutputDirectory $stage
  if ($LASTEXITCODE -ne 0) { throw 'RAW helper build failed' }
  Copy-Item (Join-Path $root 'engine/native/LICENSE-Compositor.txt') (Join-Path $stage 'LICENSE-Compositor.txt')
  @(
    "Compositor for Windows $version ($marker)",
    'Portable build: run Compositor.exe. Requires the Microsoft Edge WebView2 Runtime (preinstalled on Windows 10 19045 and later).',
    'Projects are .comp folders compatible with Compositor for macOS.'
  ) | Set-Content -Path (Join-Path $stage 'README.txt') -Encoding ascii
  Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
  $size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
  Step "Portable zip: $zip ($size MB)"

  if (-not $NoSmoke) {
    Step 'Smoke launch'
    $p = Start-Process -FilePath (Join-Path $stage 'Compositor.exe') -WindowStyle Hidden -PassThru
    Start-Sleep -Seconds 5
    if ($p.HasExited) { throw "Compositor.exe exited early with code $($p.ExitCode)" }
    Stop-Process -Id $p.Id -Force
    Step 'Smoke launch OK'
  }
  Write-Output $zip
}
finally {
  Set-Location $root
  [IO.File]::WriteAllBytes($buildInfoPath, $originalBuildInfo)
}
