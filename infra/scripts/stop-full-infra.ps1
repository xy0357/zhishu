[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$composeFile = Join-Path $repoRoot "infra\docker-compose.yml"

Push-Location $repoRoot
try {
    Write-Host "停止 MySQL / Redis / Qdrant / MinIO 基础依赖..." -ForegroundColor Cyan
    docker compose -f $composeFile down
}
finally {
    Pop-Location
}
