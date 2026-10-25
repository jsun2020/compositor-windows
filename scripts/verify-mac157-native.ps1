[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$PortableDirectory,[Parameter(Mandatory=$true)][string]$Marker,[Parameter(Mandatory=$true)][string]$Probes,[Parameter(Mandatory=$true)][string]$OutputDirectory,[Parameter(Mandatory=$true)][string]$RawFile,[int]$Port=19307,[string]$DesktopGate)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$taskOutput=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $taskOutput){throw 'Preserve old evidence; use a fresh output directory'}
if(Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue){throw 'Native test port is in use; preserve its owner'}
New-Item -ItemType Directory -Path $taskOutput|Out-Null
if($DesktopGate){
  $taskDesktop=& $DesktopGate check
  $taskDesktopExit=$LASTEXITCODE
  [IO.File]::WriteAllText((Join-Path $taskOutput 'desktop-before.json'),($taskDesktop -join [Environment]::NewLine))
  if($taskDesktopExit -ne 0){throw 'Native acceptance requires a single display and unlocked desktop; no application was launched'}
}
$taskOldArgs=$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$taskOldData=$env:WEBVIEW2_USER_DATA_FOLDER
$taskProcess=$null
$taskExit=1
try {
  $taskExe=(Resolve-Path -LiteralPath (Join-Path $PortableDirectory 'Compositor.exe')).Path
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=$Port"
  $env:WEBVIEW2_USER_DATA_FOLDER=Join-Path $taskOutput 'webview-profile'
  @{exe=$taskExe;exeSHA256=(Get-FileHash -LiteralPath $taskExe).Hash;marker=$Marker;helperSHA256=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'verify-mac157-native.cjs')).Hash;dialogHelperSHA256=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'native-dialog-control.py')).Hash;inputReceiptSHA256=(Get-FileHash -LiteralPath (Join-Path $Probes 'source-receipt.json')).Hash}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $taskOutput 'identity.json') -Encoding utf8
  $taskProcess=Start-Process -FilePath $taskExe -WindowStyle Hidden -PassThru
  $taskReady=$false
  $taskDeadline=[DateTime]::UtcNow.AddSeconds(45)
  while([DateTime]::UtcNow -lt $taskDeadline){
    if($taskProcess.HasExited){throw "Native application exited: $($taskProcess.ExitCode)"}
    try{$taskTargets=Invoke-RestMethod -Uri "http://127.0.0.1:$Port/json/list" -TimeoutSec 2;if(@($taskTargets|Where-Object{$_.type -eq 'page' -and $_.url -like '*tauri.localhost*' -and $_.title -eq 'Compositor'}).Count){$taskReady=$true;break}}catch{}
    Start-Sleep -Milliseconds 500
  }
  if(-not $taskReady){throw 'Native WebView did not start within 45 seconds'}
  $taskOldErrorAction=$ErrorActionPreference
  try{
    # Windows PowerShell treats stderr as a terminating error under Stop. Let
    # the protocol finish writing its diagnostic files, then preserve its exit.
    $ErrorActionPreference='Continue'
    & node.exe (Join-Path $PSScriptRoot 'verify-mac157-native.cjs') $Marker $taskOutput $Port $taskProcess.Id $taskExe ([IO.Path]::GetFullPath($Probes)) ([IO.Path]::GetFullPath($RawFile)) *> (Join-Path $taskOutput 'native.log')
    $taskExit=$LASTEXITCODE
  }finally{$ErrorActionPreference=$taskOldErrorAction}
  if($DesktopGate){
    $taskDesktop=& $DesktopGate check
    $taskDesktopExit=$LASTEXITCODE
    [IO.File]::WriteAllText((Join-Path $taskOutput 'desktop-after.json'),($taskDesktop -join [Environment]::NewLine))
    if($taskDesktopExit -ne 0){$taskExit=1;Write-Output 'Desktop gate failed; protocol evidence is retained and native acceptance is incomplete'}
  }
  Write-Output "Native Mac 1.5.7 increment exit $taskExit; evidence $taskOutput"
}finally{
  if($taskProcess -and -not $taskProcess.HasExited){Stop-Process -Id $taskProcess.Id -Force}
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=$taskOldArgs
  $env:WEBVIEW2_USER_DATA_FOLDER=$taskOldData
}
exit $taskExit
