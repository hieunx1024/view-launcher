# Quick Install Script for View Launcher on Windows (Portable)
$ErrorActionPreference = "Stop"

Write-Host "Installing View Launcher..." -ForegroundColor Cyan

$InstallDir = "$env:LOCALAPPDATA\Programs\view-launcher"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$CurrentDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Copy binary and icons
Copy-Item "$CurrentDir\view-launcher.exe" -Destination "$InstallDir\view-launcher.exe" -Force
if (Test-Path "$CurrentDir\view-launcher.ico") {
    Copy-Item "$CurrentDir\view-launcher.ico" -Destination "$InstallDir\view-launcher.ico" -Force
}

# Add to User PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    Write-Host "Added $InstallDir to User PATH." -ForegroundColor Green
}

$WshShell = New-Object -ComObject WScript.Shell

# 1. Create Startup Shortcut with Windows Terminal & Hotkey Ctrl+Alt+Space
$StartupDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup"
$Shortcut = $WshShell.CreateShortcut("$StartupDir\ViewLauncher.lnk")
$Shortcut.TargetPath = "wt.exe"
$Shortcut.Arguments = "-d . `"$InstallDir\view-launcher.exe`""
$Shortcut.WorkingDirectory = "$InstallDir"
if (Test-Path "$InstallDir\view-launcher.ico") {
    $Shortcut.IconLocation = "$InstallDir\view-launcher.ico, 0"
}
$Shortcut.Hotkey = "Ctrl+Alt+Space"
$Shortcut.Save()

# 2. Create Desktop Shortcut
$DesktopDir = [Environment]::GetFolderPath("Desktop")
$DeskShortcut = $WshShell.CreateShortcut("$DesktopDir\View Launcher.lnk")
$DeskShortcut.TargetPath = "wt.exe"
$DeskShortcut.Arguments = "-d . `"$InstallDir\view-launcher.exe`""
$DeskShortcut.WorkingDirectory = "$InstallDir"
if (Test-Path "$InstallDir\view-launcher.ico") {
    $DeskShortcut.IconLocation = "$InstallDir\view-launcher.ico, 0"
}
$DeskShortcut.Save()

# 3. Create Start Menu Shortcut
$StartMenuDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs"
$StartShortcut = $WshShell.CreateShortcut("$StartMenuDir\View Launcher.lnk")
$StartShortcut.TargetPath = "wt.exe"
$StartShortcut.Arguments = "-d . `"$InstallDir\view-launcher.exe`""
$StartShortcut.WorkingDirectory = "$InstallDir"
if (Test-Path "$InstallDir\view-launcher.ico") {
    $StartShortcut.IconLocation = "$InstallDir\view-launcher.ico, 0"
}
$StartShortcut.Save()

Write-Host "View Launcher installed successfully to $InstallDir!" -ForegroundColor Green
Write-Host "App is now ready! Press Ctrl + Alt + Space anywhere to launch." -ForegroundColor Yellow
