$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Install-Earnest {
    $releasesUrl = 'https://github.com/cpeaustriajc/earnest/releases/download'
    $defaultVersion = 'v0.1.0-nightly'
    $version = if ($env:EARNEST_VERSION) { $env:EARNEST_VERSION } else { $defaultVersion }
    $downloadUrl = if ($env:EARNEST_DOWNLOAD_URL) { $env:EARNEST_DOWNLOAD_URL } else { "$releasesUrl/$version" }
    $installDir = if ($env:EARNEST_INSTALL_DIR) { $env:EARNEST_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\earnest' }

    $cpu = $env:PROCESSOR_ARCHITECTURE
    if ($cpu -ne 'AMD64' -and $cpu -ne 'ARM64') {
        throw "earnest install: no nightly build for the cpu $cpu"
    }
    $archive = 'earnest-win32-x64-msvc.zip'

    $work = Join-Path ([System.IO.Path]::GetTempPath()) ("earnest-" + [System.Guid]::NewGuid())
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        Write-Host "Downloading $downloadUrl/$archive"
        Invoke-WebRequest -UseBasicParsing -Uri "$downloadUrl/$archive" -OutFile (Join-Path $work $archive)
        Invoke-WebRequest -UseBasicParsing -Uri "$downloadUrl/$archive.sha256" -OutFile (Join-Path $work "$archive.sha256")

        $expected = ((Get-Content -Raw (Join-Path $work "$archive.sha256")).Trim() -split '\s+')[0]
        $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $work $archive)).Hash
        if (-not $expected) {
            throw "earnest install: $archive.sha256 is empty"
        }
        if ($expected -ne $actual) {
            throw "earnest install: $archive has SHA-256 $actual, but $archive.sha256 says $expected"
        }

        Expand-Archive -Path (Join-Path $work $archive) -DestinationPath (Join-Path $work 'unpacked')
        New-Item -ItemType Directory -Force -Path $installDir | Out-Null
        $binary = Join-Path $installDir 'earnest.exe'
        Copy-Item -Force (Join-Path $work 'unpacked\earnest.exe') $binary

        Write-Host "Installed $(& $binary --version) in $installDir"
        $onPath = ($env:PATH -split ';') | Where-Object { $_.TrimEnd('\') -eq $installDir.TrimEnd('\') }
        if (-not $onPath) {
            Write-Host "Add $installDir to PATH to run earnest by name."
        }
    }
    finally {
        Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
    }
}

Install-Earnest
