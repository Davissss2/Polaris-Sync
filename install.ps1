#Requires -Version 5.1
<#
.SYNOPSIS
    Installs the Polaris CLI on Windows.

.DESCRIPTION
    Downloads the latest (or requested) release from GitHub, installs
    `polaris.exe` and adds it to the current user's PATH.

.EXAMPLE
    irm https://raw.githubusercontent.com/Davissss2/Polaris-Sync/main/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Version v0.1.0 -NoPath
#>
[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$Dir = (Join-Path $env:LOCALAPPDATA "Polaris\bin"),
    [switch]$NoPath
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "Davissss2/Polaris-Sync"

function Write-Step($Message) {
    Write-Host "==> " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Get-Architecture {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -or $env:PROCESSOR_ARCHITEW6432 -eq "ARM64") {
        return "aarch64"
    }

    return "x86_64"
}

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
} catch {
    # Newer runtimes default to TLS 1.2, nothing to do
}

$label = "windows-$(Get-Architecture)"

Write-Step "Resolving release ($Version) for $label"

$api = if ($Version -eq "latest") {
    "https://api.github.com/repos/$Repo/releases/latest"
} else {
    "https://api.github.com/repos/$Repo/releases/tags/$Version"
}

try {
    $release = Invoke-RestMethod -Uri $api -UseBasicParsing -Headers @{ "User-Agent" = "polaris-installer" }
} catch {
    throw "Could not reach GitHub to resolve the release. Check your connection or pass -Version explicitly. ($($_.Exception.Message))"
}

$asset = $release.assets | Where-Object { $_.name -like "*$label*.zip" } | Select-Object -First 1

if (-not $asset) {
    $available = ($release.assets | ForEach-Object { $_.name }) -join ", "
    throw "No build for $label in release $($release.tag_name). Assets: $available"
}

$temp = Join-Path ([IO.Path]::GetTempPath()) ("polaris-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $temp | Out-Null

try {
    $zip = Join-Path $temp "polaris.zip"

    Write-Step "Downloading $($asset.name)"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -UseBasicParsing

    Write-Step "Extracting"
    Expand-Archive -Path $zip -DestinationPath $temp -Force

    $exe = Get-ChildItem -Path $temp -Filter "polaris.exe" -Recurse | Select-Object -First 1

    if (-not $exe) {
        throw "The downloaded archive does not contain polaris.exe"
    }

    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    $target = Join-Path $Dir "polaris.exe"

    if (Get-Process -Name "polaris" -ErrorAction SilentlyContinue) {
        Write-Warning "A running polaris process may lock the binary. Stop it first if the copy fails."
    }

    Copy-Item -Path $exe.FullName -Destination $target -Force

    Write-Step "Installed $target"

    if (-not $NoPath) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

        if (-not $userPath) {
            $userPath = ""
        }

        $parts = $userPath -split ";" | Where-Object { $_ -ne "" }

        if ($parts -notcontains $Dir) {
            $newPath = (@($parts) + $Dir) -join ";"
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
            Write-Step "Added $Dir to the user PATH (open a new terminal to use it)"
        }

        $env:Path = "$env:Path;$Dir"
    }

    Write-Host ""
    & $target --version
    Write-Host ""
    Write-Step "Done. Try 'polaris init my-game'"
} finally {
    Remove-Item -Path $temp -Recurse -Force -ErrorAction SilentlyContinue
}
