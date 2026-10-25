[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
# The same recipe works from a repository or from the shipped HEIF-notices folder.
$standalone=Test-Path -LiteralPath (Join-Path $PSScriptRoot 'heif-sources.json')
$root=if($standalone){$PSScriptRoot}else{(Resolve-Path (Join-Path $PSScriptRoot '..')).Path}
$source=if($standalone){$PSScriptRoot}else{Join-Path $root 'src-tauri\native\heif'}
$cache=Join-Path $root 'build-artifacts\heif-native-cache'
$downloads=if($standalone){$PSScriptRoot}else{Join-Path $root 'build-artifacts\heif-deps'}
New-Item -ItemType Directory -Path $cache,$downloads -Force|Out-Null
$target=[IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $target -Force|Out-Null
$spec=Get-Content -LiteralPath (Join-Path $source 'heif-sources.json') -Raw|ConvertFrom-Json
function Hash([string]$p){(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant()}
foreach($name in @('libde265','libheif')){
 $entry=$spec.$name;$archive=Join-Path $downloads $entry.archive
 if(-not(Test-Path -LiteralPath $archive)){Invoke-WebRequest -Uri $entry.url -OutFile $archive}
 if((Hash $archive) -ne $entry.sha256){throw "Pinned $name source archive hash mismatch"}
}
$inputs=[ordered]@{script=(Hash $PSCommandPath);cpp=(Hash (Join-Path $source 'heif-decode.cpp'));cmake=(Hash (Join-Path $source 'CMakeLists.txt'));manifest=(Hash (Join-Path $source 'heif-sources.json'));readme=(Hash (Join-Path $source 'README.txt'))}
$runtime=Join-Path $cache 'runtime';$receipt=Join-Path $cache 'receipt.json';$reuse=$false
if(Test-Path -LiteralPath $receipt){
 try{
  $prior=Get-Content -LiteralPath $receipt -Raw|ConvertFrom-Json
  $same=$true;foreach($k in $inputs.Keys){if($prior.inputs.$k -ne $inputs[$k]){$same=$false}}
  foreach($row in $prior.files){$file=Join-Path $runtime $row.path;if(-not(Test-Path -LiteralPath $file) -or (Hash $file) -ne $row.sha256){$same=$false}}
  if($same -and @($prior.files).Count -eq 16){$reuse=$true}
 }catch{$reuse=$false}
}
if(-not $reuse){
 $vswhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
 $install=& $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath|Select-Object -First 1
 if(-not $install){throw 'Visual Studio C++ tools not found'}
 $dev=Join-Path $install 'Common7\Tools\VsDevCmd.bat'
 $dump=& cmd.exe /d /s /c "`"$dev`" -arch=x64 -host_arch=x64 >nul && set"
 foreach($line in $dump){if($line -match '^(.*?)=(.*)$'){[Environment]::SetEnvironmentVariable($matches[1],$matches[2],'Process')}}
 $cmakeCommand=Get-Command cmake.exe -ErrorAction SilentlyContinue
 $cmake=if($cmakeCommand){$cmakeCommand.Source}else{Join-Path $install 'Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe'}
 if(-not(Test-Path -LiteralPath $cmake)){
  # Portable build tool only: no installer, administrator access or system changes.
  $cmakeArchive=Join-Path $downloads 'cmake-3.31.10-windows-x86_64.zip'
  if(-not(Test-Path -LiteralPath $cmakeArchive)){Invoke-WebRequest -Uri 'https://github.com/Kitware/CMake/releases/download/v3.31.10/cmake-3.31.10-windows-x86_64.zip' -OutFile $cmakeArchive}
  if((Hash $cmakeArchive) -ne '13d1a463d7130df5339baedd63d8ae990aaf385062b2f42f372796143ae94086'){throw 'Portable CMake hash mismatch'}
  $cmake=Join-Path $cache 'cmake-3.31.10-windows-x86_64\bin\cmake.exe'
  Expand-Archive -LiteralPath $cmakeArchive -DestinationPath $cache -Force
 }
 $ninja=Join-Path $install 'Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe'
 $generator=@('-G','NMake Makefiles')
 if(Test-Path -LiteralPath $ninja){$generator=@('-G','Ninja',"-DCMAKE_MAKE_PROGRAM=$ninja")}
 function Invoke-CMake([Parameter(Mandatory=$true)][string[]]$CMakeArguments){
  if($CMakeArguments.Count -lt 2){throw 'CMake requires an explicitly bound argument array'}
  & $cmake @CMakeArguments
  if($LASTEXITCODE -ne 0){throw "CMake failed: $CMakeArguments"}
 }
 $work=Join-Path $cache ('build-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory -Path $work|Out-Null
 $prefix=Join-Path $work 'install';$fresh=Join-Path $work 'runtime';New-Item -ItemType Directory -Path $fresh|Out-Null
 foreach($name in @('libde265','libheif')){& tar.exe -xf (Join-Path $downloads $spec.$name.archive) -C $work;if($LASTEXITCODE -ne 0){throw "Cannot extract $name"}}
 $de265=Join-Path $work ('libde265-'+$spec.libde265.version);$heif=Join-Path $work ('libheif-'+$spec.libheif.version)
 Invoke-CMake -CMakeArguments (@('-S',$de265,'-B',(Join-Path $work 'de265-build'))+$generator+@('-DCMAKE_BUILD_TYPE=Release',"-DCMAKE_INSTALL_PREFIX=$prefix",'-DBUILD_SHARED_LIBS=ON','-DENABLE_DECODER=OFF','-DENABLE_ENCODER=OFF','-DENABLE_SDL=OFF','-DENABLE_AVX512=OFF'))
 Invoke-CMake -CMakeArguments @('--build',(Join-Path $work 'de265-build'),'--config','Release','--parallel','2')
 Invoke-CMake -CMakeArguments @('--install',(Join-Path $work 'de265-build'),'--config','Release')
 Invoke-CMake -CMakeArguments (@('-S',$heif,'-B',(Join-Path $work 'heif-build'))+$generator+@('-DCMAKE_BUILD_TYPE=Release',"-DCMAKE_INSTALL_PREFIX=$prefix","-DCMAKE_PREFIX_PATH=$prefix",'-DBUILD_SHARED_LIBS=ON','-DENABLE_PLUGIN_LOADING=OFF','-DWITH_LIBDE265=ON','-DWITH_LIBDE265_PLUGIN=OFF','-DWITH_X265=OFF','-DWITH_X264=OFF','-DWITH_OpenH264_DECODER=OFF','-DWITH_AOM_DECODER=OFF','-DWITH_AOM_ENCODER=OFF','-DWITH_DAV1D=OFF','-DWITH_LIBSHARPYUV=OFF','-DWITH_EXAMPLES=OFF','-DWITH_GDK_PIXBUF=OFF','-DBUILD_TESTING=OFF','-DBUILD_DOCUMENTATION=OFF','-DENABLE_PARALLEL_TILE_DECODING=OFF'))
 Invoke-CMake -CMakeArguments @('--build',(Join-Path $work 'heif-build'),'--config','Release','--parallel','2')
 Invoke-CMake -CMakeArguments @('--install',(Join-Path $work 'heif-build'),'--config','Release')
 Invoke-CMake -CMakeArguments (@('-S',$source,'-B',(Join-Path $work 'helper-build'))+$generator+@('-DCMAKE_BUILD_TYPE=Release',"-DCMAKE_PREFIX_PATH=$prefix","-DCMAKE_INSTALL_PREFIX=$prefix"))
 Invoke-CMake -CMakeArguments @('--build',(Join-Path $work 'helper-build'),'--config','Release','--parallel','2')
 Invoke-CMake -CMakeArguments @('--install',(Join-Path $work 'helper-build'),'--config','Release')
 foreach($name in @('CompositorHeif.exe','heif.dll','libde265.dll')){Copy-Item -LiteralPath (Join-Path $prefix "bin\$name") -Destination $fresh}
 $crtRoot=Join-Path $install 'VC\Redist\MSVC'
 $crt=Get-ChildItem -LiteralPath $crtRoot -Directory|Where-Object {$_.Name -match '^\d+\.\d+\.\d+$'}|Sort-Object {[version]$_.Name} -Descending|Select-Object -First 1
 if(-not $crt){throw 'MSVC runtime redistributables not found'}
 foreach($name in @('msvcp140.dll','vcruntime140.dll','vcruntime140_1.dll')){Copy-Item -LiteralPath (Join-Path $crt.FullName "x64\Microsoft.VC143.CRT\$name") -Destination $fresh}
 $notice=Join-Path $fresh 'HEIF-notices';New-Item -ItemType Directory -Path $notice|Out-Null
 foreach($name in @('heif-decode.cpp','heif-sources.json','CMakeLists.txt','README.txt')){Copy-Item -LiteralPath (Join-Path $source $name) -Destination $notice}
 Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $notice 'build-heif-helper.ps1')
 foreach($name in @('libde265','libheif')){
  Copy-Item -LiteralPath (Join-Path $downloads $spec.$name.archive) -Destination $notice
  Copy-Item -LiteralPath (Join-Path $work "$name-$($spec.$name.version)\COPYING") -Destination (Join-Path $notice "$name-COPYING.txt")
 }
 $components=@();foreach($name in @('CompositorHeif.exe','heif.dll','libde265.dll')){$components+=@{name=$name;sha256=(Hash (Join-Path $fresh $name))}}
 @{schema=1;sources=$spec;inputs=$inputs;components=$components;outputBits=16;systemHevcRequired=$false;decoder='libde265';plugins=$false;msvcRuntimeVersion=$crt.Name}|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $notice 'provenance.json') -Encoding UTF8
 # Include complete corresponding sources and a rebuild recipe; never ship a system-codec dependency.
 if(Test-Path -LiteralPath $runtime){Move-Item -LiteralPath $runtime -Destination (Join-Path $cache ('previous-runtime-'+[Guid]::NewGuid().ToString('N')))}
 Move-Item -LiteralPath $fresh -Destination $runtime
 $files=@(Get-ChildItem -LiteralPath $runtime -File -Recurse|ForEach-Object {@{path=$_.FullName.Substring($runtime.Length+1).Replace('\','/');sha256=(Hash $_.FullName)}})
 @{inputs=$inputs;files=$files}|ConvertTo-Json -Depth 6|Set-Content -LiteralPath $receipt -Encoding UTF8
}
Copy-Item -Path (Join-Path $runtime '*') -Destination $target -Recurse -Force
Write-Host "Bundled HEIF decoder ready: $target (verified cache: $reuse)"
