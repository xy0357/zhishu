[CmdletBinding()]
param(
    [string]$MySqlHost = "127.0.0.1",
    [int]$MySqlPort = 3306,
    [string]$RedisHost = "127.0.0.1",
    [int]$RedisPort = 6379,
    [string]$QdrantUrl = "http://127.0.0.1:6333/healthz",
    [string]$MinioApiHost = "127.0.0.1",
    [int]$MinioApiPort = 9000,
    [string]$MinioConsoleHost = "127.0.0.1",
    [int]$MinioConsolePort = 9001
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Test-TcpEndpoint {
    param(
        [string]$HostName,
        [int]$Port
    )

    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $asyncResult = $client.BeginConnect($HostName, $Port, $null, $null)
        if (-not $asyncResult.AsyncWaitHandle.WaitOne(1000)) {
            return $false
        }
        $client.EndConnect($asyncResult) | Out-Null
        return $true
    }
    catch {
        return $false
    }
    finally {
        $client.Dispose()
    }
}

function Get-CommandStatus {
    param(
        [string]$Name,
        [string]$Command,
        [string[]]$Args
    )

    $resolved = Get-Command $Command -ErrorAction SilentlyContinue
    if (-not $resolved) {
        return [PSCustomObject]@{
            Name = $Name
            Status = "missing"
            Detail = "未安装或未加入 PATH"
        }
    }

    $detail = try {
        & $Command @Args 2>$null | Select-Object -First 1
    }
    catch {
        "已安装，但版本读取失败"
    }

    [PSCustomObject]@{
        Name = $Name
        Status = "ok"
        Detail = $detail
    }
}

$commands = @(
    (Get-CommandStatus -Name "cargo" -Command "cargo.exe" -Args @("--version")),
    (Get-CommandStatus -Name "npm" -Command "npm.cmd" -Args @("--version")),
    (Get-CommandStatus -Name "mysql" -Command "mysql" -Args @("--version"))
)

Write-Host "路线 B（无 Docker）命令检查：" -ForegroundColor Cyan
$commands | Format-Table -AutoSize

$qdrantStatus = try {
    $response = Invoke-WebRequest -Uri $QdrantUrl -UseBasicParsing -TimeoutSec 2
    if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 500) { "reachable" } else { "closed" }
}
catch {
    "closed"
}

$services = @(
    [PSCustomObject]@{
        Name = "mysql:$MySqlPort"
        Status = if (Test-TcpEndpoint -HostName $MySqlHost -Port $MySqlPort) { "reachable" } else { "closed" }
        Detail = "本机真实数据库"
    },
    [PSCustomObject]@{
        Name = "redis:$RedisPort"
        Status = if (Test-TcpEndpoint -HostName $RedisHost -Port $RedisPort) { "reachable" } else { "closed" }
        Detail = "可使用 Redis 兼容实现，例如 Memurai"
    },
    [PSCustomObject]@{
        Name = "qdrant"
        Status = $qdrantStatus
        Detail = "可使用外部 Qdrant 服务或后续再接入"
    },
    [PSCustomObject]@{
        Name = "minio-api:$MinioApiPort"
        Status = if (Test-TcpEndpoint -HostName $MinioApiHost -Port $MinioApiPort) { "reachable" } else { "closed" }
        Detail = "当前代码仍可先走 OBJECT_STORAGE_DIR 本地镜像"
    },
    [PSCustomObject]@{
        Name = "minio-console:$MinioConsolePort"
        Status = if (Test-TcpEndpoint -HostName $MinioConsoleHost -Port $MinioConsolePort) { "reachable" } else { "closed" }
        Detail = "仅用于人工管理对象存储"
    }
)

Write-Host ""
Write-Host "路线 B 服务连通性：" -ForegroundColor Cyan
$services | Format-Table -AutoSize

$requiredMissing = @()
$requiredMissing += @($commands | Where-Object { $_.Status -ne "ok" })
$requiredMissing += @($services | Where-Object { $_.Name -eq "mysql:$MySqlPort" -and $_.Status -ne "reachable" })

Write-Host ""
if ($requiredMissing) {
    Write-Host "当前还不能完整执行路线 B。至少需要满足：cargo、npm、mysql 命令可用，且 MySQL 端口可达。" -ForegroundColor Yellow
    Write-Host "Redis / Qdrant / MinIO 在当前仓库中仍属于可选增强依赖，可后续逐步补齐。" -ForegroundColor Yellow
    exit 1
}

Write-Host "路线 B 基础检查通过，可继续执行 MySQL 模式后端与前端联调。" -ForegroundColor Green
