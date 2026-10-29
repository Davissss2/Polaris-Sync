#Requires -Version 5.1
<#
.SYNOPSIS
    Installs Polaris on Windows: the CLI and the Roblox Studio plugin.

.DESCRIPTION
    Downloads a release from GitHub, installs `polaris.exe` under
    %LOCALAPPDATA%\Polaris\bin, adds it to the user PATH and drops
    `Polaris.rbxm` in the Roblox Studio plugins folder. Everything is per user,
    so it never asks for administrator rights.

    Restart Roblox Studio after installing: it only reads the plugins folder
    when it starts.

.EXAMPLE
    .\PolarisSetup.exe

.EXAMPLE
    .\PolarisSetup.exe -Version v0.1.1 -NoPlugin

.EXAMPLE
    .\PolarisSetup.exe -Uninstall
#>
[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$Dir = (Join-Path $env:LOCALAPPDATA "Polaris\bin"),
    [switch]$NoPath,
    [switch]$NoPlugin,
    [switch]$Force,
    [switch]$Uninstall,
    [switch]$Silent
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$Repo = "Davissss2/Polaris-Sync"
$PluginName = "Polaris.rbxm"

function Write-Step($Message) {
    Write-Host "  ==> " -ForegroundColor Cyan -NoNewline
    Write-Host $Message
}

function Write-Note($Message) {
    Write-Host "      " -NoNewline
    Write-Host $Message -ForegroundColor DarkGray
}

function Write-Ok($Message) {
    Write-Host "  [ok] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Fail($Message) {
    Write-Host "  [!!] " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

function Show-Header {
    Write-Host ""
    Write-Host "  POLARIS" -ForegroundColor White -NoNewline
    Write-Host "  Roblox sync tool" -ForegroundColor DarkGray
    Write-Host "  ------------------------------------------" -ForegroundColor DarkGray
    Write-Host ""
}

function Pause-If-Interactive {
    if ($Silent) {
        return
    }

    Write-Host ""
    Write-Host "  Pulsa Enter para cerrar..." -ForegroundColor DarkGray -NoNewline
    Read-Host | Out-Null
}

function Get-Architecture {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64" -or $env:PROCESSOR_ARCHITEW6432 -eq "ARM64") {
        return "aarch64"
    }

    return "x86_64"
}

function Get-PluginsDir {
    return (Join-Path $env:LOCALAPPDATA "Roblox\Plugins")
}

function Resolve-Release {
    $api = if ($Version -eq "latest") {
        "https://api.github.com/repos/$Repo/releases/latest"
    } else {
        "https://api.github.com/repos/$Repo/releases/tags/$Version"
    }

    return Invoke-RestMethod -Uri $api -UseBasicParsing -Headers @{ "User-Agent" = "polaris-setup" }
}

function Remove-FromUserPath([string]$Entry) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if (-not $userPath) {
        return $false
    }

    $parts = $userPath -split ";" | Where-Object { $_ -ne "" -and $_ -ne $Entry }

    if ($parts.Count -eq ($userPath -split ";" | Where-Object { $_ -ne "" }).Count) {
        return $false
    }

    [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")

    return $true
}

function Add-ToUserPath([string]$Entry) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if (-not $userPath) {
        $userPath = ""
    }

    $parts = $userPath -split ";" | Where-Object { $_ -ne "" }

    if ($parts -contains $Entry) {
        return $false
    }

    [Environment]::SetEnvironmentVariable("Path", ((@($parts) + $Entry) -join ";"), "User")

    return $true
}

function Uninstall-Polaris {
    Show-Header
    Write-Step "Uninstalling Polaris"

    foreach ($name in @("Polaris.rbxm")) {
        if ($NoPlugin) {
            continue
        }

        $pluginPath = Join-Path (Get-PluginsDir) $name

        if (Test-Path $pluginPath) {
            Remove-Item $pluginPath -Force
            Write-Ok "Removed the Studio plugin ($pluginPath)"
            Write-Note "Restart Roblox Studio to unload it"
        }
    }

    $binPath = Join-Path $Dir "polaris.exe"

    if (Test-Path $binPath) {
        $running = Get-Process -Name "polaris" -ErrorAction SilentlyContinue

        if ($running) {
            $running | Stop-Process -Force
            Write-Note "Stopped running polaris sessions"
        }

        if (Test-Path $Dir) {
            Remove-Item $Dir -Recurse -Force -ErrorAction SilentlyContinue
        }

        if (Test-Path $binPath) {
            Write-Fail "Could not remove $binPath (is it still running?)"
        } else {
            Write-Ok "Removed $Dir"
        }
    }

    if (Remove-FromUserPath $Dir) {
        Write-Ok "Removed $Dir from the user PATH"
    }

    Write-Host ""
    Write-Ok "Polaris is uninstalled"
    Write-Note "Your projects on disk were not touched"

    return
}

function Install-Polaris {
    Show-Header

    $label = "windows-$(Get-Architecture)"

    Write-Step "Resolving the $Version release for $label"

    try {
        $release = Resolve-Release
    } catch {
        throw "Could not reach GitHub to resolve the release. Check your connection or pass -Version explicitly. ($($_.Exception.Message))"
    }

    $tag = $release.tag_name
    Write-Ok "Release $tag"

    $asset = $release.assets | Where-Object { $_.name -like "*$label*.zip" } | Select-Object -First 1

    if (-not $asset) {
        $available = ($release.assets | ForEach-Object { $_.name }) -join ", "
        throw "No build for $label in $tag. Assets: $available"
    }

    $temp = Join-Path ([IO.Path]::GetTempPath()) ("polaris-setup-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $temp | Out-Null

    try {
        $zip = Join-Path $temp "polaris.zip"

        Write-Step "Downloading $($asset.name)"
        Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip -UseBasicParsing
        Write-Ok ("Downloaded {0:N1} MB" -f ((Get-Item $zip).Length / 1MB))

        Write-Step "Extracting"
        Expand-Archive -Path $zip -DestinationPath $temp -Force

        $exe = Get-ChildItem -Path $temp -Filter "polaris.exe" -Recurse | Select-Object -First 1

        if (-not $exe) {
            throw "The downloaded archive does not contain polaris.exe"
        }

        if (Get-Process -Name "polaris" -ErrorAction SilentlyContinue) {
            Write-Note "A polaris session is running and may lock the binary"
        }

        New-Item -ItemType Directory -Force -Path $Dir | Out-Null
        $target = Join-Path $Dir "polaris.exe"
        Copy-Item -Path $exe.FullName -Destination $target -Force
        Write-Ok "Installed the CLI at $target"

        if (-not $NoPath) {
            if (Add-ToUserPath $Dir) {
                Write-Ok "Added $Dir to the user PATH"
                Write-Note "Open a new terminal to use 'polaris' without the full path"
            } else {
                Write-Note "$Dir was already in the user PATH"
            }

            $env:Path = "$env:Path;$Dir"
        }

        if (-not $Silent) {
            $installed = (& $target --version 2>&1 | Out-String).Trim()
            Write-Note $installed
        }

        if (-not $NoPlugin) {
            Install-Plugin $release
        }

        Write-Host ""
        Write-Ok "Polaris $tag is ready"

        if (-not $NoPlugin) {
            Write-Note "Restart Roblox Studio: it only loads plugins on startup"
        }

        Write-Note "Next: polaris init my-game && polaris serve my-game"
    } finally {
        Remove-Item -Path $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Install-Plugin($release) {
    $pluginAsset = $release.assets | Where-Object { $_.name -eq $PluginName } | Select-Object -First 1

    if (-not $pluginAsset) {
        Write-Note "Release $($release.tag_name) has no $PluginName, skipping the plugin"

        return
    }

    $pluginsDir = Get-PluginsDir
    $pluginPath = Join-Path $pluginsDir $PluginName

    if ((Test-Path $pluginPath) -and -not $Force) {
        $existing = Get-Item $pluginPath
        Write-Note "A Studio plugin is already installed ($($existing.LastWriteTime.ToString('yyyy-MM-dd HH:mm')))"

        if ($existing.LastWriteTime -gt (Get-Date).AddMinutes(-5)) {
            Write-Note "It looks like a local build, keeping it. Pass -Force to overwrite"
        } else {
            Write-Note "Keeping it. Pass -Force to replace it with the release build"
        }

        return
    }

    Write-Step "Downloading the Studio plugin"

    New-Item -ItemType Directory -Force -Path $pluginsDir | Out-Null
    Invoke-WebRequest -Uri $pluginAsset.browser_download_url -OutFile $pluginPath -UseBasicParsing

    Write-Ok "Installed the plugin at $pluginPath"
}

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
} catch {
    # Newer runtimes already default to TLS 1.2
}

try {
    if ($Uninstall) {
        Uninstall-Polaris
    } else {
        Install-Polaris
    }

    Pause-If-Interactive

    exit 0
} catch {
    Show-Header
    Write-Fail "The installation failed"

    Write-Host ""
    Write-Host "  $($_.Exception.Message)" -ForegroundColor Yellow
    Write-Host ""

    Pause-If-Interactive

    exit 1
}
