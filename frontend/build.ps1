# TaskMod Frontend Build Script (PowerShell)
# 编译 Dioxus 前端并部署到 server/static/

$ErrorActionPreference = "Stop"

$FrontendDir = $PSScriptRoot
$StaticDir = Join-Path $FrontendDir "..\server\static"
$BackupDir = Join-Path $FrontendDir "..\server\static_backup"

Write-Host "=== TaskMod Frontend Build ===" -ForegroundColor Cyan
Write-Host "Frontend dir: $FrontendDir"
Write-Host "Static dir: $StaticDir"

# 备份现有静态文件
if ((Test-Path $StaticDir) -and !(Test-Path $BackupDir)) {
    Write-Host "Backing up existing static files..." -ForegroundColor Yellow
    Copy-Item -Path $StaticDir -Destination $BackupDir -Recurse
}

# 检查 Dioxus CLI
if (!(Get-Command "dx" -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Dioxus CLI..." -ForegroundColor Yellow
    cargo install dioxus-cli
}

# 编译前端
Write-Host "Building Dioxus frontend..." -ForegroundColor Green
Set-Location $FrontendDir
dx build --release

# 复制编译产物到 static 目录
Write-Host "Deploying to static directory..." -ForegroundColor Green
New-Item -ItemType Directory -Path $StaticDir -Force | Out-Null

$DistDir = Join-Path $FrontendDir "dist"

if (Test-Path $DistDir) {
    # 清理旧文件
    Remove-Item -Path (Join-Path $StaticDir "index.html") -Force -ErrorAction SilentlyContinue
    Remove-Item -Path (Join-Path $StaticDir "style.css") -Force -ErrorAction SilentlyContinue
    Remove-Item -Path (Join-Path $StaticDir "app.js") -Force -ErrorAction SilentlyContinue

    # 复制新文件
    Copy-Item -Path "$DistDir\*" -Destination $StaticDir -Recurse -Force

    Write-Host "Build complete! Frontend deployed to $StaticDir" -ForegroundColor Green
    Write-Host ""
    Write-Host "Files deployed:" -ForegroundColor Cyan
    Get-ChildItem $StaticDir | Format-Table Name, Length, LastWriteTime
} else {
    Write-Host "Error: Build output not found at $DistDir" -ForegroundColor Red
    exit 1
}
