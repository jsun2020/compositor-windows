# Exercise the real build wrapper with a native argv reader, including PS5 legacy quoting.
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$tokens=$null;$parseErrors=$null
$ast=[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $root 'scripts/build-heif-helper.ps1'),[ref]$tokens,[ref]$parseErrors)
if($parseErrors.Count){throw ($parseErrors|Out-String)}
$fn=$ast.Find({param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Invoke-CMake'},$true)
if(-not $fn){throw 'Missing native build wrapper'}
Invoke-Expression $fn.Extent.Text
$calls=@($ast.FindAll({param($n) $n -is [System.Management.Automation.Language.CommandAst] -and $n.GetCommandName() -eq 'Invoke-CMake'},$true))
if($calls.Count -ne 9){throw 'Expected all nine configure/build/install calls'}
foreach($call in $calls){if($call.CommandElements[1].ParameterName -ne 'CMakeArguments'){throw 'Bind the argument array explicitly for Windows PowerShell 5'}}
$cmake=(Get-Command node -ErrorAction Stop).Source
$temp=Join-Path ([IO.Path]::GetTempPath()) ('heif argument probe '+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp|Out-Null
try{
 $probe=Join-Path $temp 'argv.cjs'
 [IO.File]::WriteAllText($probe,'if(process.argv[2]==="--fail")process.exit(17);if(process.argv[2]==="--success")process.exit(0);const expected=JSON.parse(require("fs").readFileSync(process.argv[2],"utf8").replace(/^\uFEFF/,""));const actual=process.argv.slice(3);console.log(JSON.stringify({expected,actual}));if(JSON.stringify(actual)!==JSON.stringify(expected))process.exit(18);')
 $expected=@('-S','C:\Users\test user\source','-B','C:\build path\output','-G','Ninja','-DCMAKE_MAKE_PROGRAM=C:\Program Files (x86)\CMake\ninja.exe','-DCMAKE_BUILD_TYPE=Release')
 $expectedFile=Join-Path $temp 'expected.json'
 [IO.File]::WriteAllText($expectedFile,($expected|ConvertTo-Json -Compress))
 # Compare inside Node: PS5 ConvertFrom-Json pipeline array enumeration differs from PS7.
 # PowerShell 5 always uses legacy native argument passing; PS7 can test both.
 foreach($mode in @('Legacy','Standard')){
  $PSNativeCommandArgumentPassing=$mode
  Invoke-CMake -CMakeArguments (@($probe,$expectedFile)+$expected)
 }
 $collapsedRejected=$false
 try{Invoke-CMake -CMakeArguments @($probe,$expectedFile,($expected -join ' '))|Out-Null}catch{$collapsedRejected=($LASTEXITCODE -eq 18)}
 if(-not $collapsedRejected){throw 'The argv reader did not reject a collapsed argument array'}
 $failed=$false
 try{Invoke-CMake -CMakeArguments @($probe,'--fail')}catch{$failed=$true}
 if(-not $failed){throw 'Native build failure was swallowed'}
 Invoke-CMake -CMakeArguments @($probe,'--success')|Out-Null
 Write-Host 'PASS: nine explicit array calls, native argument boundaries, spaced paths and failure propagation.'
}finally{Remove-Item -LiteralPath $temp -Recurse -Force}
