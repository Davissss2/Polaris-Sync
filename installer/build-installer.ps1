#Requires -Version 5.1
<#
.SYNOPSIS
    Builds dist/PolarisSetup.exe from installer/PolarisSetup.ps1.

.DESCRIPTION
    Wraps the installer script in a console executable with ps2exe and embeds
    an icon generated from the Polaris logo. The resulting exe is per user and
    needs no administrator rights.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File installer\build-installer.ps1
#>
[CmdletBinding()]
param(
    [string]$Version = "0.1.1",
    [string]$Output = "dist\PolarisSetup.exe"
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$root = Split-Path $PSScriptRoot -Parent
$source = Join-Path $PSScriptRoot "PolarisSetup.ps1"
$output = Join-Path $root $Output
$iconPath = Join-Path $root "dist\polaris.ico"

function New-AppIcon([string]$pngPath, [string]$icoPath, [int]$size = 256) {
    $src = [System.Drawing.Image]::FromFile($pngPath)
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)

    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.DrawImage($src, 0, 0, $size, $size)
    $g.Dispose()
    $src.Dispose()

    $stream = New-Object System.IO.MemoryStream
    $bmp.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    $png = $stream.ToArray()
    $stream.Dispose()
    $bmp.Dispose()

    # ICO container with a single PNG payload (supported since Windows Vista)
    $file = [IO.File]::Create($icoPath)
    $writer = New-Object System.IO.BinaryWriter($file)

    $writer.Write([UInt16]0)          # reserved
    $writer.Write([UInt16]1)          # type: icon
    $writer.Write([UInt16]1)          # image count
    $writer.Write([Byte]0)            # width: 0 means 256
    $writer.Write([Byte]0)            # height: 0 means 256
    $writer.Write([Byte]0)            # palette size
    $writer.Write([Byte]0)            # reserved
    $writer.Write([UInt16]1)          # colour planes
    $writer.Write([UInt16]32)         # bits per pixel
    $writer.Write([UInt32]$png.Length) # payload size
    $writer.Write([UInt32]22)         # payload offset
    $writer.Write($png)

    $writer.Flush()
    $writer.Dispose()
    $file.Dispose()
}

$distDir = Split-Path $output -Parent
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

New-AppIcon (Join-Path $root "assets\branding\polaris-icon-idle.png") $iconPath
Write-Host "Icon: $iconPath"

Import-Module ps2exe -Force

Invoke-PS2EXE -InputFile $source `
    -OutputFile $output `
    -IconFile $iconPath `
    -X64 `
    -STA `
    -Title "Polaris Setup" `
    -Description "Installs the Polaris CLI and the Roblox Studio plugin" `
    -Company "Polaris" `
    -Product "Polaris" `
    -Version $Version `
    -Copyright "Apache-2.0"

$file = Get-Item $output
Write-Host ("Built {0} ({1:N0} KB)" -f $file.FullName, ($file.Length / 1KB))
