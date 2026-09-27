# Quick Install Script for View Launcher on Windows (Portable)
$ErrorActionPreference = "Stop"

Write-Host "Installing View Launcher..." -ForegroundColor Cyan

$InstallDir = "$env:LOCALAPPDATA\Programs\view-launcher"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$CurrentDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Locate binary
$SourceExe = "$CurrentDir\view-launcher.exe"
if (-not (Test-Path $SourceExe)) {
    $RepoExe = Join-Path $CurrentDir "..\..\target\release\view-launcher.exe"
    if (Test-Path $RepoExe) {
        $SourceExe = (Resolve-Path $RepoExe).Path
    } else {
        throw "view-launcher.exe not found at $SourceExe or $RepoExe. Please run 'cargo build --release' first."
    }
}

# Locate icon
$SourceIco = "$CurrentDir\view-launcher.ico"
if (-not (Test-Path $SourceIco)) {
    $RepoIco = Join-Path $CurrentDir "..\..\assets\view-launcher.ico"
    if (Test-Path $RepoIco) {
        $SourceIco = (Resolve-Path $RepoIco).Path
    }
}

# Stop running instance if any to avoid file lock
Get-Process -Name "view-launcher" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

# Copy binary and icons
Copy-Item $SourceExe -Destination "$InstallDir\view-launcher.exe" -Force
if (Test-Path $SourceIco) {
    Copy-Item $SourceIco -Destination "$InstallDir\view-launcher.ico" -Force
}

# Add to User PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to User PATH." -ForegroundColor Green
}

$WshShell = New-Object -ComObject WScript.Shell

# 1. Create Startup Shortcut
$StartupDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$Shortcut = $WshShell.CreateShortcut("$StartupDir\ViewLauncher.lnk")
$Shortcut.TargetPath = "$InstallDir\view-launcher.exe"
$Shortcut.Arguments = ""
$Shortcut.WorkingDirectory = "$InstallDir"
if (Test-Path "$InstallDir\view-launcher.ico") {
    $Shortcut.IconLocation = "$InstallDir\view-launcher.ico, 0"
}
$Shortcut.Hotkey = "Ctrl+Alt+Space"
$Shortcut.Save()

# 2. Create Desktop Shortcut
$DesktopDir = [Environment]::GetFolderPath("Desktop")
$DeskShortcut = $WshShell.CreateShortcut("$DesktopDir\View Launcher.lnk")
$DeskShortcut.TargetPath = "$InstallDir\view-launcher.exe"
$DeskShortcut.Arguments = ""
$DeskShortcut.WorkingDirectory = "$InstallDir"
if (Test-Path "$InstallDir\view-launcher.ico") {
    $DeskShortcut.IconLocation = "$InstallDir\view-launcher.ico, 0"
}
$DeskShortcut.Save()

# 3. Create Start Menu Shortcut
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$StartShortcut = $WshShell.CreateShortcut("$StartMenuDir\View Launcher.lnk")
$StartShortcut.TargetPath = "$InstallDir\view-launcher.exe"
$StartShortcut.Arguments = ""
$StartShortcut.WorkingDirectory = "$InstallDir"
if (Test-Path "$InstallDir\view-launcher.ico") {
    $StartShortcut.IconLocation = "$InstallDir\view-launcher.ico, 0"
}
$StartShortcut.Save()

Write-Host "View Launcher installed successfully to $InstallDir!" -ForegroundColor Green
Write-Host "App is now ready! Press Alt + Z anywhere to launch." -ForegroundColor Yellow
