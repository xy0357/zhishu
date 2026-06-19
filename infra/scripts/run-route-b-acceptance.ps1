[CmdletBinding()]
param(
    [string]$BaseUrl = "http://127.0.0.1:8080/api",
    [string]$AdminUsername = "admin",
    [string]$AdminPassword = "Admin@123456"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)

Push-Location $repoRoot
try {
    $effectiveQdrantUrl = if ($env:QDRANT_URL) { $env:QDRANT_URL } else { $null }

    Write-Host "路线 B 验收开始：" -ForegroundColor Cyan
    Write-Host "1. 检查无 Docker 基础环境" -ForegroundColor DarkCyan
    powershell -ExecutionPolicy Bypass -File "infra\scripts\check-route-b-prerequisites.ps1"

    Write-Host ""
    Write-Host "2. 执行 MySQL 主链路验收" -ForegroundColor DarkCyan
    powershell -ExecutionPolicy Bypass -File "infra\scripts\run-mysql-acceptance.ps1" `
        -BaseUrl $BaseUrl `
        -AdminUsername $AdminUsername `
        -AdminPassword $AdminPassword

    Write-Host ""
    Write-Host "3. 执行扩展全量验收" -ForegroundColor DarkCyan
    powershell -ExecutionPolicy Bypass -File "infra\scripts\run-full-acceptance.ps1" `
        -BaseUrl $BaseUrl `
        -AdminUsername $AdminUsername `
        -AdminPassword $AdminPassword

    Write-Host ""
    Write-Host "4. 检查对象存储镜像一致性" -ForegroundColor DarkCyan
    powershell -ExecutionPolicy Bypass -File "infra\scripts\check-object-storage-mirror.ps1" `
        -BaseUrl $BaseUrl `
        -AdminUsername $AdminUsername `
        -AdminPassword $AdminPassword

    $health = Invoke-RestMethod -Method Get -Uri "$BaseUrl/health"
    if (-not $effectiveQdrantUrl) {
        $effectiveQdrantUrl = $health.data.dependencies.qdrant.configured
    }

    $shouldRunQdrantDemo = $false
    if ($env:QDRANT_URL -and $env:QDRANT_API_KEY) {
        $shouldRunQdrantDemo = $true
    }
    elseif ($health.data.dependencies.qdrant.reachable) {
        $shouldRunQdrantDemo = $true
    }

    if ($shouldRunQdrantDemo) {
        Write-Host ""
        Write-Host "5. 检测到 Qdrant 可达，执行 demo 向量同步与校验" -ForegroundColor DarkCyan
        if ($env:QDRANT_URL) {
            Write-Host ("   使用环境变量 QDRANT_URL = {0}" -f $env:QDRANT_URL) -ForegroundColor DarkCyan
        }
        powershell -ExecutionPolicy Bypass -File "infra\scripts\export-qdrant-demo-points.ps1" `
            -BaseUrl $BaseUrl `
            -AdminUsername $AdminUsername `
            -AdminPassword $AdminPassword
        powershell -ExecutionPolicy Bypass -File "infra\scripts\sync-qdrant-demo.ps1" `
            -QdrantUrl $effectiveQdrantUrl `
            -QdrantApiKey $env:QDRANT_API_KEY
        powershell -ExecutionPolicy Bypass -File "infra\scripts\run-qdrant-demo-check.ps1" `
            -QdrantUrl $effectiveQdrantUrl `
            -QdrantApiKey $env:QDRANT_API_KEY
        powershell -ExecutionPolicy Bypass -File "infra\scripts\search-qdrant-demo.ps1" `
            -QdrantUrl $effectiveQdrantUrl `
            -QdrantApiKey $env:QDRANT_API_KEY
    }
    else {
        Write-Host ""
        Write-Host "5. 当前未检测到可用的 Qdrant 配置，跳过 demo 向量同步检查。" -ForegroundColor Yellow
        Write-Host "   如使用 Qdrant Cloud，请先在当前终端设置 QDRANT_URL 与 QDRANT_API_KEY。" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "路线 B 验收完成。" -ForegroundColor Green
}
finally {
    Pop-Location
}
