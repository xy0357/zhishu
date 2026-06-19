[CmdletBinding()]
param(
    [switch]$Build
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$composeFile = Join-Path $repoRoot "infra\docker-compose.yml"

Push-Location $repoRoot
try {
    $args = @("compose", "-f", $composeFile, "up", "-d")
    if ($Build) {
        $args += "--build"
    }

    Write-Host "启动 MySQL / Redis / Qdrant / MinIO 基础依赖..." -ForegroundColor Cyan
    docker @args

    Write-Host ""
    Write-Host "基础依赖已提交启动，请继续执行：" -ForegroundColor Green
    Write-Host "1. infra/scripts/start-mysql-backend.ps1" -ForegroundColor DarkCyan
    Write-Host "2. frontend 下 npm run dev" -ForegroundColor DarkCyan
    Write-Host "3. infra/scripts/run-full-acceptance.ps1" -ForegroundColor DarkCyan
}
finally {
    Pop-Location
}
