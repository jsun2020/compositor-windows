[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$PortableDirectory,
  [Parameter(Mandatory=$true)][string]$Marker,
  [Parameter(Mandatory=$true)][string]$Probes,
  [Parameter(Mandatory=$true)][string]$OutputDirectory,
  [Parameter(Mandatory=$true)][string]$RawFile,
  [Parameter(Mandatory=$true)][string]$DesktopGate,
  [int]$Port=19311
)
# Mac 1.5.11 bounded native group launcher (preparation only until Mac authorizes).
# Patterned on verify-mac157-native.ps1: real WebView2, private profile, PID-scoped dialogs,
# reject existing output / occupied port, DesktopGate before/after, restore only owned env,
# stop only launched process. Does not change company lock policy; no keepawake/RN.
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$taskOutput=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $taskOutput){throw 'Preserve old evidence; use a fresh output directory'}
if(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue){throw 'Native test port is in use; preserve its owner'}
if(-not (Test-Path -LiteralPath $DesktopGate)){throw "DesktopGate missing: $DesktopGate"}
if(-not (Test-Path -LiteralPath $RawFile)){throw "RawFile missing (kept for launcher parity): $RawFile"}
New-Item -ItemType Directory -Path $taskOutput|Out-Null
$taskDesktop=& $DesktopGate check
$taskDesktopExit=$LASTEXITCODE
[IO.File]::WriteAllText((Join-Path $taskOutput 'desktop-before.json'),($taskDesktop -join [Environment]::NewLine))
if($taskDesktopExit -ne 0){throw 'Native acceptance requires a single display and unlocked desktop; no application was launched'}
$taskOldArgs=$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$taskOldData=$env:WEBVIEW2_USER_DATA_FOLDER
$taskProcess=$null
$taskExit=1
$taskNode=$null
try {
  $taskExe=(Resolve-Path -LiteralPath (Join-Path $PortableDirectory 'Compositor.exe')).Path
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=$Port"
  $env:WEBVIEW2_USER_DATA_FOLDER=Join-Path $taskOutput 'webview-profile'
  @{exe=$taskExe;exeSHA256=(Get-FileHash -LiteralPath $taskExe).Hash;marker=$Marker;helperSHA256=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'verify-mac1511-native.cjs')).Hash;dialogHelperSHA256=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'native-dialog-control.py')).Hash;inputReceiptSHA256=(Get-FileHash -LiteralPath (Join-Path $Probes 'source-receipt.json')).Hash;scope='MAC1511_BOUNDED_NATIVE_PREP';acceptanceClaim=$false}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $taskOutput 'identity.json') -Encoding utf8
  $taskProcess=Start-Process -FilePath $taskExe -WindowStyle Hidden -PassThru
  $taskReady=$false
  $taskDeadline=[DateTime]::UtcNow.AddSeconds(45)
  while([DateTime]::UtcNow -lt $taskDeadline){
    if($taskProcess.HasExited){throw "Native application exited: $($taskProcess.ExitCode)"}
    try{$taskTargets=Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/list" -TimeoutSec 2;if(@($taskTargets|Where-Object{$_.type -eq 'page' -and $_.url -like '*tauri.localhost*' -and $_.title -eq 'Compositor'}).Count){$taskReady=$true;break}}catch{}
    Start-Sleep -Milliseconds 500
  }
  if(-not $taskReady){throw 'Native WebView did not start within 45 seconds'}
  # Bound the complete group without widening individual UI assertions.
  $taskNodeArgs=@((Join-Path $PSScriptRoot 'verify-mac1511-native.cjs'),$Marker,$taskOutput,$Port,$taskProcess.Id,$taskExe,([IO.Path]::GetFullPath($Probes)),([IO.Path]::GetFullPath($RawFile)))
  $taskQuoted=($taskNodeArgs|ForEach-Object{'"'+([string]$_).Replace('"','\"')+'"'}) -join ' '
  $taskNode=Start-Process -FilePath (Get-Command node.exe).Source -ArgumentList $taskQuoted -RedirectStandardOutput (Join-Path $taskOutput 'native.stdout.log') -RedirectStandardError (Join-Path $taskOutput 'native.stderr.log') -WindowStyle Hidden -PassThru
  # Retain the process handle before waiting so Windows PowerShell can read the
  # actual exit status even after the process has terminated.
  $taskNodeHandle=$taskNode.Handle
  if(-not $taskNode.WaitForExit(480000)){throw 'Native group exceeded eight minutes; preserve failed evidence'}
  $taskNode.WaitForExit()
  $taskExit=$taskNode.ExitCode
  @{pid=$taskNode.Id;exitCode=$taskExit;capturedUtc=[DateTime]::UtcNow.ToString('o');source='retained Process handle after WaitForExit'}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $taskOutput 'native-process-exit.json') -Encoding utf8
  if($null -eq $taskExit){throw 'Native process exit code unavailable; refuse to infer success'}
  $taskDesktop=& $DesktopGate check
  $taskDesktopExit=$LASTEXITCODE
  [IO.File]::WriteAllText((Join-Path $taskOutput 'desktop-after.json'),($taskDesktop -join [Environment]::NewLine))
  if($taskDesktopExit -ne 0){$taskExit=1;Write-Output 'Desktop gate failed; protocol evidence is retained and native acceptance is incomplete'}
  Write-Output "Native Mac 1.5.11 bounded group exit $taskExit; evidence $taskOutput"
}finally{
  if($taskNode -and -not $taskNode.HasExited){& taskkill.exe /PID $taskNode.Id /T /F | Out-Null}
  if($taskProcess -and -not $taskProcess.HasExited){Stop-Process -Id $taskProcess.Id -Force}
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=$taskOldArgs
  $env:WEBVIEW2_USER_DATA_FOLDER=$taskOldData
}
exit $taskExit
