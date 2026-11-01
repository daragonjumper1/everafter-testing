$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $projectDirectory = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))
    $localRust = Join-Path $projectDirectory 'work\rust-install'
    $env:RUSTUP_HOME = Join-Path $localRust 'rustup'
    $env:CARGO_HOME = Join-Path $localRust 'cargo'
    $env:PATH = (Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-msvc\bin') + ';' + $env:PATH
}
cargo build --manifest-path Cargo.toml --target-dir ..\target --bin everafter-desktop
if ($LASTEXITCODE -ne 0) { throw 'Windows build failed.' }
Copy-Item -LiteralPath '..\target\debug\everafter-desktop.exe' -Destination '..\..\Everafter.exe' -Force
Write-Host 'Built outputs\Everafter.exe. Double-click it to open the desktop app.'
