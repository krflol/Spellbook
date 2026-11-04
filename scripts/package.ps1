param(
    [string]$TargetDir = "target",
    [string]$OutputDir = "dist",
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if (-not $SkipBuild) {
        cargo build --locked --release --target-dir $TargetDir
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    }
    $manifest = Get-Content -LiteralPath 'Cargo.toml' -Raw
    $version = [regex]::Match($manifest, '(?m)^version = "([^"]+)"').Groups[1].Value
    if (-not $version) { throw 'Package version not found' }
    $name = "Spellbook-$version-windows-x64"
    $stage = Join-Path $OutputDir $name
    $archive = Join-Path $OutputDir "$name.zip"
    if ((Test-Path -LiteralPath $stage) -or (Test-Path -LiteralPath $archive)) {
        throw 'Release output exists. Choose a fresh -OutputDir to avoid mixing releases.'
    }
    $exe = Join-Path $TargetDir 'release/spellbook.exe'
    if (-not (Test-Path -LiteralPath $exe)) { throw "Missing executable: $exe" }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath $exe -Destination $stage
    foreach ($file in @('LICENSE','THIRD_PARTY_NOTICES.txt','CHANGELOG.md')) {
        Copy-Item -LiteralPath $file -Destination $stage
    }
    Copy-Item -LiteralPath 'docs/GUIDE.md' -Destination (Join-Path $stage 'USER-GUIDE.md')
    Copy-Item -LiteralPath 'assets/catalogue-example.json' -Destination $stage
    @'
Spellbook — World of Warcraft macro IDE

Open spellbook.exe to start. Your library is stored in %LOCALAPPDATA%\Spellbook.
For portable storage beside the executable, run: spellbook.exe --portable
For a custom data folder, run: spellbook.exe --data-dir "D:\My macros"

Create a macro or choose a sample, then Copy macro and paste it into WoW.
Use File to import/export; Workspace > Settings to customize the editor.
F1 opens help and keyboard shortcuts. See USER-GUIDE.md for details.

This is unsigned community software. Windows may display a publisher warning.
Samples require adapting to your character and are not live-game-tested.

Source, documentation, updates: https://github.com/krflol/Spellbook
No installer, admin rights, account, or internet connection is required.
'@ | Set-Content -LiteralPath (Join-Path $stage 'START-HERE.txt') -Encoding UTF8
    $exeHash = (Get-FileHash -LiteralPath (Join-Path $stage 'spellbook.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    "$exeHash  spellbook.exe" | Set-Content -LiteralPath (Join-Path $stage 'SHA256SUMS.txt') -Encoding Ascii
    Compress-Archive -LiteralPath $stage -DestinationPath $archive -CompressionLevel Optimal
    $zipHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    "$zipHash  $name.zip" | Set-Content -LiteralPath (Join-Path $OutputDir 'SHA256SUMS.txt') -Encoding Ascii
    Write-Output "Packaged $archive"
    Write-Output "SHA-256 $zipHash"
} finally {
    Pop-Location
}
