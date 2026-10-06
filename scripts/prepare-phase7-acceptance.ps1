[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$OutputDirectory,[string]$PsdFile)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$taskOutput=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $taskOutput){throw 'Preserve existing acceptance evidence; choose a fresh output directory.'}
New-Item -ItemType Directory -Path $taskOutput|Out-Null
Push-Location $taskRoot
try {
  & cargo run -p compositor-engine --example phase7_camera_probes -- (Join-Path $taskOutput 'CameraRaw')
  if($LASTEXITCODE -ne 0){throw 'Camera Raw probe generation failed'}
  & python -X utf8 scripts/make-phase7-psd-probes.py (Join-Path $taskOutput 'Photoshop')
  if($LASTEXITCODE -ne 0){throw 'Photoshop probe generation failed'}
  & python -X utf8 scripts/make-phase7-dng-probe.py (Join-Path $taskOutput 'bayer-no-preview.dng')
  if($LASTEXITCODE -ne 0){throw 'DNG probe generation failed'}
  Copy-Item -LiteralPath (Join-Path $taskRoot 'docs/superpowers/plans/2026-10-06-phase7-mac-checks.md') -Destination (Join-Path $taskOutput 'MAC-STEPS.md')
  Copy-Item -LiteralPath (Join-Path $taskRoot 'docs/superpowers/plans/2026-10-06-phase7-windows-checks.md') -Destination (Join-Path $taskOutput 'WINDOWS-STEPS.md')
  @('Mac version:','Compositor version:','Camera Raw cases 01 / 02 / 03 / 04:','Preview / Cancel / one Undo+Redo / reopen:','White balance / Point Color / diagnostics / group controls:','PSD and PSB structure and appearance:','Editable text and rectangle after save/reopen:','Adjustment conversion messages:','Real PSD import:','Synthetic DNG accepted or rejected:','Real camera RAW file and camera model:','RAW preview / Cancel / Reset / full Import / reopen:','Failures or missing controls:')|Set-Content -LiteralPath (Join-Path $taskOutput 'RESULT.txt') -Encoding utf8
  $photoshop=@(Get-ChildItem -LiteralPath (Join-Path $taskOutput 'Photoshop') -File|Where-Object{$_.Extension -in @('.psd','.psb')})
  foreach($file in $photoshop){
    $output=Join-Path $taskOutput "Windows-imports/$($file.BaseName)-$($file.Extension.Substring(1))"
    & cargo run -p compositor-engine --example phase7_psd_probe -- $file.FullName $output
    if($LASTEXITCODE -ne 0){throw "Synthetic Photoshop import failed: $($file.Name)"}
  }
  if($PsdFile){
    $original=(Resolve-Path -LiteralPath $PsdFile).Path;$before=(Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash
    New-Item -ItemType Directory -Path (Join-Path $taskOutput 'Real-PSD')|Out-Null
    $copy=Join-Path $taskOutput 'Real-PSD/source.psd';Copy-Item -LiteralPath $original -Destination $copy
    & cargo run -p compositor-engine --example phase7_psd_probe -- $copy (Join-Path $taskOutput 'Windows-imports/real-psd')
    if($LASTEXITCODE -ne 0){throw 'Real Photoshop import failed'}
    if((Get-FileHash -LiteralPath $original -Algorithm SHA256).Hash -ne $before){throw 'Original Photoshop source changed'}
  }
  $hashes=@(Get-ChildItem -LiteralPath $taskOutput -File -Recurse|ForEach-Object{@{name=$_.FullName.Substring($taskOutput.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash}})
  @{schema=1;oracle='Compositor 1.4.5';generated=(Get-Date).ToString('o');sourceCommit=(& git rev-parse HEAD);sourceDirty=[bool](& git status --porcelain --untracked-files=no);files=$hashes}|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $taskOutput 'input-receipt.json') -Encoding utf8
  Write-Output "Acceptance probes prepared: $taskOutput"
} finally {Pop-Location}
