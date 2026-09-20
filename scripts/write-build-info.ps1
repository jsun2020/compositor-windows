param([string]$Version = "")
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Version) { $Version = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version }
$stamp = Get-Date -Format 'yyyyMMdd-HHmm'
$marker = "COMPOSITOR_BUILD_${Version}_$stamp"
Set-Content -Path (Join-Path $root 'app\src\build-info.ts') -Value "export const BUILD_MARKER = `"$marker`";" -Encoding ascii
Write-Output $marker
