[CmdletBinding()]
param(
    [string]$AppHost = "127.0.0.1",
    [int]$AppPort = 8080,
    [string]$StorageFile = "data/demo-store.json"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$backendDir = Join-Path $repoRoot "backend"

Push-Location $backendDir
try {
    $env:APP_HOST = $AppHost
    $env:APP_PORT = $AppPort.ToString()
    $env:APP_STORAGE_BACKEND = "file"
    $env:APP_STORAGE_FILE = $StorageFile

    Write-Host "以 file 模式启动后端：$AppHost`:$AppPort" -ForegroundColor Cyan
    cargo.exe run
}
finally {
    Pop-Location
}
