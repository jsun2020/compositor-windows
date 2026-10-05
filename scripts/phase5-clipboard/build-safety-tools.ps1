param([string]$OutputDirectory)
$ErrorActionPreference='Stop'
if(-not $env:WINDIR){throw 'These diagnostic tools require Windows'}
$taskRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
if(-not $OutputDirectory){$OutputDirectory=Join-Path $taskRoot ('build-artifacts/phase5-acceptance/safety-tools-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))}
$taskOutput=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $taskOutput){throw 'Output directory already exists; previous evidence is preserved'}
$taskCompiler=Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
if(-not(Test-Path -LiteralPath $taskCompiler)){$taskCompiler=Join-Path $env:WINDIR 'Microsoft.NET/Framework/v4.0.30319/csc.exe'}
if(-not(Test-Path -LiteralPath $taskCompiler)){throw '.NET Framework C# compiler not found'}
New-Item -ItemType Directory -Path $taskOutput | Out-Null
$taskManifest=@()
foreach($taskName in @('ClipboardOwnershipProbe','ClipboardSnapshotGuard','ClipboardReadOnlyLocker','ClipboardTextNativeRecovery')){
 $taskSource=Join-Path $PSScriptRoot ($taskName+'.cs')
 $taskExe=Join-Path $taskOutput ($taskName+'.exe')
 $taskArguments=@('/nologo','/target:exe',('/out:'+$taskExe),'/reference:System.Web.Extensions.dll')
 if($taskName -ne 'ClipboardOwnershipProbe'){$taskArguments+=@('/reference:System.Windows.Forms.dll','/reference:System.Drawing.dll')}
 if($taskName -eq 'ClipboardSnapshotGuard' -or $taskName -eq 'ClipboardTextNativeRecovery'){$taskArguments+='/reference:System.Security.dll'}
 $taskArguments+=$taskSource
 & $taskCompiler @taskArguments
 if($LASTEXITCODE -ne 0){throw "Compiler failed: $taskName"}
 $taskManifest+=[pscustomobject]@{source=($taskName+'.cs');sourceSha256=(Get-FileHash -LiteralPath $taskSource -Algorithm SHA256).Hash;executable=($taskName+'.exe');executableSha256=(Get-FileHash -LiteralPath $taskExe -Algorithm SHA256).Hash}
}
$taskManifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOutput 'build-inputs.json') -Encoding UTF8
Write-Output "Safety tools built without clipboard access: $taskOutput"
