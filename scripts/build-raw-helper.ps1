[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$archive=Join-Path $taskRoot 'build-artifacts\LibRaw-0.22.2-Win64.zip'
$sdkParent=Join-Path $taskRoot 'build-artifacts\phase7-libraw-0.22.2'
$sdk=Join-Path $sdkParent 'LibRaw-0.22.2'
$expected='AC64FA12BB00A7581332D4C6AB918C0533FB3F119D6B668D47A6875410DCA948'
if(-not(Test-Path -LiteralPath $archive)){New-Item -ItemType Directory -Path (Split-Path $archive) -Force|Out-Null;Invoke-WebRequest -Uri 'https://www.libraw.org/data/LibRaw-0.22.2-Win64.zip' -OutFile $archive}
if((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expected){throw 'Pinned LibRaw SDK hash mismatch'}
# Restore the SDK from its verified archive before compiling; edited SDK files are never trusted.
Expand-Archive -LiteralPath $archive -DestinationPath $sdkParent -Force
$taskOutput=[IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $taskOutput -Force|Out-Null
$vswhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$install=& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath|Select-Object -First 1
if(-not $install){throw 'Visual Studio C++ tools not found'}
$dev=Join-Path $install 'Common7\Tools\VsDevCmd.bat'
$dump=& cmd.exe /d /s /c "`"$dev`" -arch=x64 -host_arch=x64 >nul && set"
foreach($line in $dump){if($line -match '^(.*?)=(.*)$'){[Environment]::SetEnvironmentVariable($matches[1],$matches[2],'Process')}}
$source=Join-Path $taskRoot 'src-tauri\native\raw-develop.cpp'
$exe=Join-Path $taskOutput 'CompositorRaw.exe'
$obj=Join-Path $sdkParent 'raw-develop.obj'
& cl.exe /nologo /EHsc /std:c++17 /utf-8 /O2 /MD /DWIN32 /D_WINDOWS /DLIBRAW_NODLL "$source" "/I$sdk" "/Fo$obj" "/Fe$exe" /link (Join-Path $sdk 'lib\libraw_static.lib')
if($LASTEXITCODE -ne 0){throw 'RAW helper compilation failed'}
# LibRaw is static; its MSVC runtime dependencies are shipped beside the helper.
$crtRoot=Join-Path $install 'VC\Redist\MSVC'
$crtVersion=Get-ChildItem -LiteralPath $crtRoot -Directory | Where-Object {$_.Name -match '^\d+\.\d+\.\d+$'} | Sort-Object {[version]$_.Name} -Descending | Select-Object -First 1
if(-not $crtVersion){throw 'Redistributable x64 MSVC runtime not found'}
$crtDirectory=Join-Path $crtVersion.FullName 'x64\Microsoft.VC143.CRT'
$crtReceipt=@()
foreach($name in @('msvcp140.dll','vcruntime140.dll','vcruntime140_1.dll')){
  $crtFile=Join-Path $crtDirectory $name
  if(-not(Test-Path -LiteralPath $crtFile)){throw "Missing redistributable: $name"}
  Copy-Item -LiteralPath $crtFile -Destination (Join-Path $taskOutput $name)
  $crtReceipt+=@{name=$name;sha256=(Get-FileHash -LiteralPath $crtFile -Algorithm SHA256).Hash}
}
$notice=Join-Path $taskOutput 'LibRaw-notices'
New-Item -ItemType Directory -Path $notice -Force|Out-Null
foreach($name in @('COPYRIGHT','LICENSE.CDDL','LICENSE.LGPL')){Copy-Item -LiteralPath (Join-Path $sdk $name) -Destination (Join-Path $notice $name)}
@{version='0.22.2';license='CDDL-1.0';source='https://www.libraw.org/data/LibRaw-0.22.2-Win64.zip';sourceSHA256=$expected;helperSHA256=(Get-FileHash -LiteralPath $exe).Hash;msvcRuntimeVersion=$crtVersion.Name;msvcRuntime=$crtReceipt}|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $notice 'provenance.json') -Encoding utf8
Write-Output $exe
