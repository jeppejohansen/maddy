# Install maddy, a Markdown-and-mathematics to PDF compiler.
#
#   irm https://raw.githubusercontent.com/jeppejohansen/maddy/main/install.ps1 | iex
#
# Environment:
#   MADDY_VERSION      a tag to install, such as v0.1.0 (default: the latest)
#   MADDY_INSTALL_DIR  where to put the binary (default: %LOCALAPPDATA%\maddy\bin)
#
# The binary is self-contained: nothing else is installed, and no Typst or LaTeX
# toolchain is required.

$ErrorActionPreference = 'Stop'

$repo = 'jeppejohansen/maddy'
$installDir = if ($env:MADDY_INSTALL_DIR) {
    $env:MADDY_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA 'maddy\bin'
}

if ([Environment]::Is64BitOperatingSystem -eq $false) {
    throw 'maddy is only published for 64-bit Windows; build from source with `cargo install --git https://github.com/jeppejohansen/maddy`'
}

# --- which version ---------------------------------------------------------

$version = $env:MADDY_VERSION
if (-not $version) {
    Write-Host 'Looking up the latest release...'
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $version = $release.tag_name
}
if (-not $version) {
    throw 'Could not determine the latest release; set MADDY_VERSION to install a specific one.'
}
$number = $version -replace '^v', ''

# --- download --------------------------------------------------------------

$name = "maddy-$number-x86_64-windows"
$archive = "$name.zip"
$base = "https://github.com/$repo/releases/download/$version"
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $temp -Force | Out-Null

try {
    Write-Host "Downloading $archive..."
    $zip = Join-Path $temp $archive
    Invoke-WebRequest "$base/$archive" -OutFile $zip -UseBasicParsing

    # The release publishes one SHA256SUMS covering every archive.
    try {
        $sums = (Invoke-WebRequest "$base/SHA256SUMS" -UseBasicParsing).Content
        $expected = ($sums -split "`n" |
            Where-Object { $_ -match [regex]::Escape($archive) } |
            ForEach-Object { ($_ -split '\s+')[0] } |
            Select-Object -First 1)
        if ($expected) {
            $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
            if ($actual -ne $expected.ToLower()) {
                throw "Checksum mismatch for $archive; refusing to install."
            }
            Write-Host 'Checksum verified.'
        }
    } catch [System.Net.WebException] {
        # No checksum file published; the download still stands on HTTPS.
    }

    Expand-Archive -Path $zip -DestinationPath (Join-Path $temp 'unpacked') -Force

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    $binary = Join-Path $installDir 'maddy.exe'
    Move-Item -Path (Join-Path $temp 'unpacked\maddy.exe') -Destination $binary -Force

    $reported = & $binary --version
    Write-Host ''
    Write-Host "Installed $reported to $binary"

    # Put it on PATH for future sessions if it is not already there.
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($userPath -notlike "*$installDir*") {
        [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
        Write-Host ''
        Write-Host "Added $installDir to your PATH. Open a new terminal to use ``maddy``."
    } else {
        Write-Host 'Run `maddy paper.md` to compile a document.'
    }
} finally {
    Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
}
