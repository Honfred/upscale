# Скачивает и раскладывает sidecar-бинарники (realesrgan-ncnn-vulkan,
# rife-ncnn-vulkan, ffmpeg, ffprobe) в src-tauri/binaries/ с именами
# <name>-<target-triple>[.exe] — как того требует Tauri externalBin
# (см. src-tauri/tauri.conf.json -> bundle.externalBin).
#
# ВАЖНО: значения версий/URL продублированы из scripts/versions.sh
# (единственный источник правды для *nix-скриптов). При обновлении версий
# там — обнови и здесь.
#
# Использование:
#   pwsh scripts/fetch-sidecars.ps1 [-TargetTriple <triple>] [-Force]
#
# TargetTriple по умолчанию берётся из `rustc -Vv`.
# Поддерживается: x86_64-pc-windows-msvc (а также x86_64-unknown-linux-gnu,
# если запускать pwsh на Linux — для симметрии с bash-версией).

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$TargetTriple,

    [switch]$Force
)

$ErrorActionPreference = "Stop"

# --- Пиновые версии (держать в синхроне с scripts/versions.sh) ---
$RealesrganTag           = "v0.2.5.0"
$RealesrganLinuxAsset    = "realesrgan-ncnn-vulkan-20220424-ubuntu.zip"
$RealesrganWindowsAsset  = "realesrgan-ncnn-vulkan-20220424-windows.zip"
$RealesrganBaseUrl       = "https://github.com/xinntao/Real-ESRGAN/releases/download/$RealesrganTag"

$RifeTag                 = "20221029"
$RifeLinuxAsset          = "rife-ncnn-vulkan-20221029-ubuntu.zip"
$RifeWindowsAsset        = "rife-ncnn-vulkan-20221029-windows.zip"
$RifeBaseUrl              = "https://github.com/nihui/rife-ncnn-vulkan/releases/download/$RifeTag"
$RifeLinuxExtractDir      = "rife-ncnn-vulkan-20221029-ubuntu"
$RifeWindowsExtractDir    = "rife-ncnn-vulkan-20221029-windows"

$FfmpegTag                = "autobuild-2026-07-13-14-11"
$FfmpegLinuxAsset         = "ffmpeg-n7.1.5-2-g998de74adf-linux64-gpl-7.1.tar.xz"
$FfmpegWindowsAsset       = "ffmpeg-n7.1.5-2-g998de74adf-win64-gpl-7.1.zip"
$FfmpegBaseUrl            = "https://github.com/BtbN/FFmpeg-Builds/releases/download/$FfmpegTag"
$FfmpegLinuxExtractDir    = "ffmpeg-n7.1.5-2-g998de74adf-linux64-gpl-7.1"
$FfmpegWindowsExtractDir  = "ffmpeg-n7.1.5-2-g998de74adf-win64-gpl-7.1"

function Write-Log {
    param([string]$Message)
    Write-Host "[fetch-sidecars] $Message"
}

if (-not $TargetTriple) {
    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        $hostLine = (& rustc -Vv) | Select-String '^host: '
        if ($hostLine) {
            $TargetTriple = $hostLine.ToString().Substring("host: ".Length).Trim()
        }
    }
}
if (-not $TargetTriple) {
    Write-Error "Не удалось определить target-triple (rustc не найден). Передайте -TargetTriple явно."
    exit 1
}

switch ($TargetTriple) {
    "x86_64-pc-windows-msvc"   { $Platform = "windows"; $ExeExt = ".exe" }
    "x86_64-unknown-linux-gnu" { $Platform = "linux";   $ExeExt = "" }
    default {
        Write-Error "Неподдерживаемый target-triple: $TargetTriple (поддерживаются x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu)"
        exit 1
    }
}

$RootDir       = Split-Path -Parent $PSScriptRoot
$CacheDir      = Join-Path $RootDir ".cache"
$DownloadDir   = Join-Path $CacheDir "downloads"
$ExtractDir    = Join-Path $CacheDir "sidecars-extract"
$BinDir        = Join-Path $RootDir "src-tauri/binaries"

New-Item -ItemType Directory -Force -Path $DownloadDir, $ExtractDir, $BinDir | Out-Null

function Get-CachedFile {
    param([string]$Url, [string]$Dest)
    if ((Test-Path $Dest) -and (-not $Force)) {
        Write-Log "кэш есть, пропуск скачивания: $(Split-Path -Leaf $Dest)"
        return
    }
    Write-Log "скачивание: $Url"
    $tmp = "$Dest.part"
    Invoke-WebRequest -Uri $Url -OutFile $tmp -UseBasicParsing
    Move-Item -Force $tmp $Dest
}

function Expand-ZipMembers {
    param([string]$Archive, [string]$Dest, [string[]]$Members)
    New-Item -ItemType Directory -Force -Path $Dest | Out-Null
    if ($Members -and $Members.Count -gt 0) {
        # .NET ZipFile умеет выбирать записи по имени — распаковываем только нужное.
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [System.IO.Compression.ZipFile]::OpenRead($Archive)
        try {
            foreach ($member in $Members) {
                $entry = $zip.Entries | Where-Object { $_.FullName -eq $member } | Select-Object -First 1
                if (-not $entry) {
                    throw "В архиве $Archive не найдена запись: $member"
                }
                $outPath = Join-Path $Dest $member
                New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outPath) | Out-Null
                [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $outPath, $true)
            }
        }
        finally {
            $zip.Dispose()
        }
    }
    else {
        Expand-Archive -Path $Archive -DestinationPath $Dest -Force
    }
}

function Set-Sidecar {
    param([string]$Src, [string]$Name)
    $dest = Join-Path $BinDir "$Name-$TargetTriple$ExeExt"
    if ((Test-Path $dest) -and (-not $Force)) {
        Write-Log "бинарник уже на месте, пропуск: $(Split-Path -Leaf $dest)"
        return
    }
    Copy-Item -Force $Src $dest
    Write-Log "готово: $(Split-Path -Leaf $dest)"
}

### 1. Real-ESRGAN ncnn-vulkan ###
if ($Platform -eq "linux") {
    $reAsset = $RealesrganLinuxAsset
    $reBinName = "realesrgan-ncnn-vulkan"
}
else {
    $reAsset = $RealesrganWindowsAsset
    $reBinName = "realesrgan-ncnn-vulkan.exe"
}
$reZip = Join-Path $DownloadDir $reAsset
Get-CachedFile -Url "$RealesrganBaseUrl/$reAsset" -Dest $reZip

$reExtract = Join-Path $ExtractDir "realesrgan-$Platform"
if ($Force -or -not (Test-Path (Join-Path $reExtract $reBinName))) {
    Remove-Item -Recurse -Force $reExtract -ErrorAction SilentlyContinue
    Expand-ZipMembers -Archive $reZip -Dest $reExtract -Members @($reBinName)
}
Set-Sidecar -Src (Join-Path $reExtract $reBinName) -Name "realesrgan-ncnn-vulkan"
# realesrgan-ncnn-vulkan.exe динамически линкуется с vcomp140.dll (OpenMP из
# MSVC redist); в архиве DLL лежит рядом с exe (запись "vcomp140.dll" в корне).
# Кладём её в binaries/ — src-tauri/tauri.windows.conf.json бандлит её через
# bundle.resources рядом с exe приложения.
if ($Platform -eq "windows") {
    $vcompDest = Join-Path $BinDir "vcomp140.dll"
    if ($Force -or -not (Test-Path $vcompDest)) {
        Expand-ZipMembers -Archive $reZip -Dest $reExtract -Members @("vcomp140.dll")
        Copy-Item -Force (Join-Path $reExtract "vcomp140.dll") $vcompDest
        Write-Log "готово: vcomp140.dll"
    }
    else {
        Write-Log "vcomp140.dll уже на месте, пропуск"
    }
}

### 2. RIFE ncnn-vulkan ###
if ($Platform -eq "linux") {
    $rifeAsset = $RifeLinuxAsset
    $rifeTopDir = $RifeLinuxExtractDir
    $rifeBinName = "rife-ncnn-vulkan"
}
else {
    $rifeAsset = $RifeWindowsAsset
    $rifeTopDir = $RifeWindowsExtractDir
    $rifeBinName = "rife-ncnn-vulkan.exe"
}
$rifeZip = Join-Path $DownloadDir $rifeAsset
Get-CachedFile -Url "$RifeBaseUrl/$rifeAsset" -Dest $rifeZip

$rifeExtract = Join-Path $ExtractDir "rife-$Platform"
$rifeBinRelPath = "$rifeTopDir/$rifeBinName"
if ($Force -or -not (Test-Path (Join-Path $rifeExtract $rifeBinRelPath))) {
    Remove-Item -Recurse -Force $rifeExtract -ErrorAction SilentlyContinue
    Expand-ZipMembers -Archive $rifeZip -Dest $rifeExtract -Members @($rifeBinRelPath)
}
Set-Sidecar -Src (Join-Path $rifeExtract $rifeBinRelPath) -Name "rife-ncnn-vulkan"

### 3. ffmpeg + ffprobe ###
if ($Platform -eq "linux") {
    $ffAsset = $FfmpegLinuxAsset
    $ffTopDir = $FfmpegLinuxExtractDir
    $ffExeExt = ""
}
else {
    $ffAsset = $FfmpegWindowsAsset
    $ffTopDir = $FfmpegWindowsExtractDir
    $ffExeExt = ".exe"
}
$ffArchive = Join-Path $DownloadDir $ffAsset
Get-CachedFile -Url "$FfmpegBaseUrl/$ffAsset" -Dest $ffArchive

$ffExtract = Join-Path $ExtractDir "ffmpeg-$Platform"
$ffmpegRelPath = "$ffTopDir/bin/ffmpeg$ffExeExt"
$ffprobeRelPath = "$ffTopDir/bin/ffprobe$ffExeExt"
if ($Force -or -not (Test-Path (Join-Path $ffExtract $ffmpegRelPath))) {
    Remove-Item -Recurse -Force $ffExtract -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $ffExtract | Out-Null
    if ($Platform -eq "linux") {
        # .tar.xz — используем системный tar.exe (есть на Windows Server /
        # GitHub-hosted runners, поддерживает и .tar.xz, и bsdtar-режим для .zip).
        tar -xJf $ffArchive -C $ffExtract $ffmpegRelPath $ffprobeRelPath
        if ($LASTEXITCODE -ne 0) {
            throw "tar -xJf завершился с ошибкой ($LASTEXITCODE) при распаковке $ffArchive"
        }
    }
    else {
        Expand-ZipMembers -Archive $ffArchive -Dest $ffExtract -Members @($ffmpegRelPath, $ffprobeRelPath)
    }
}
Set-Sidecar -Src (Join-Path $ffExtract $ffmpegRelPath) -Name "ffmpeg"
Set-Sidecar -Src (Join-Path $ffExtract $ffprobeRelPath) -Name "ffprobe"

Write-Log "Готово. target-triple=$TargetTriple"
Get-ChildItem $BinDir | Format-Table Name, Length
