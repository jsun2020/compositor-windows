# Verifies wasm-pack is installed at the version this project is pinned to.
#
# wasm-pack cannot be an npm devDependency here: the npm package's postinstall script
# downloads a prebuilt binary from GitHub, and this project runs behind a corporate proxy
# that may block GitHub downloads while still reaching the npm registry and crates.io. So the
# pinned version lives in package.json's "config.wasmPackVersion" instead, and this script
# (run before "wasm" and "wasm:dev") checks the wasm-pack already on PATH against it.
$required = (Get-Content (Join-Path (Split-Path -Parent $PSScriptRoot) 'package.json') -Raw | ConvertFrom-Json).config.wasmPackVersion
$installCmd = "cargo install wasm-pack --version $required --locked"

$cmd = Get-Command wasm-pack -ErrorAction SilentlyContinue
if (-not $cmd) {
  Write-Error "wasm-pack is not installed. Install the pinned version with:`n  $installCmd"
  exit 1
}

try {
  $output = & wasm-pack --version 2>&1 | Out-String
} catch {
  Write-Error "Could not run 'wasm-pack --version' ($_). Install the pinned version with:`n  $installCmd"
  exit 1
}

if ($output -notmatch '(\d+\.\d+\.\d+)') {
  Write-Error "Could not parse 'wasm-pack --version' output ('$($output.Trim())'). Install the pinned version with:`n  $installCmd"
  exit 1
}
$found = $matches[1]

if ($found -ne $required) {
  Write-Error "wasm-pack $found is installed, but this project is pinned to $required. Install it with:`n  $installCmd"
  exit 1
}

Write-Output "wasm-pack $found matches the pinned version."
