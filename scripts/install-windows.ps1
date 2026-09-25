$ErrorActionPreference = 'Stop'

if ($env:OS -ne 'Windows_NT') {
    throw 'This installer only runs on Windows.'
}

$projectDir = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$releaseExe = Join-Path $projectDir 'target\release\notiz.exe'
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\Notiz'
$installedExe = Join-Path $installDir 'notiz.exe'
$startupDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Startup)
$programsDir = [Environment]::GetFolderPath([Environment+SpecialFolder]::Programs)

Push-Location -LiteralPath $projectDir
try {
    & cargo build --release
    if ($LASTEXITCODE -ne 0) {
        throw 'The release build failed. Notiz was not installed.'
    }
} finally {
    Pop-Location
}

New-Item -ItemType Directory -Path $installDir -Force | Out-Null
try {
    Copy-Item -LiteralPath $releaseExe -Destination $installedExe -Force
} catch {
    throw "Could not copy Notiz. Quit any running Notiz app from its tray menu, then try again. $($_.Exception.Message)"
}

$shell = New-Object -ComObject WScript.Shell
foreach ($shortcutPath in @(
    (Join-Path $programsDir 'Notiz v2.lnk'),
    (Join-Path $startupDir 'Notiz v2.lnk')
)) {
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $installedExe
    $shortcut.WorkingDirectory = $installDir
    $shortcut.IconLocation = "$installedExe,0"
    $shortcut.Description = 'Notiz plain text notes'
    $shortcut.Save()
}

Write-Host "Installed Notiz to $installedExe"
Write-Host 'It will start in the system tray when you next sign in to Windows.'
