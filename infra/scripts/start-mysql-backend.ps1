[CmdletBinding()]
param(
    [string]$AppHost = "127.0.0.1",
    [int]$AppPort = 8080,
    [string]$MySqlUrl = "mysql://zhishu:zhishu@127.0.0.1:3306/zhishu"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$backendDir = Join-Path $repoRoot "backend"

Push-Location $backendDir
try {
    $env:APP_HOST = $AppHost
    $env:APP_PORT = $AppPort.ToString()
    $env:APP_STORAGE_BACKEND = "mysql"
    $env:MYSQL_URL = $MySqlUrl

    Write-Host "以 mysql 模式启动后端：$AppHost`:$AppPort" -ForegroundColor Cyan
    Write-Host "启动时会自动执行 backend/migrations 下尚未执行的迁移。" -ForegroundColor DarkCyan
    cargo.exe run
}
finally {
    Pop-Location
}
