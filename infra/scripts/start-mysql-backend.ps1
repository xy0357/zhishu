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

function Test-TcpEndpoint {
    param(
        [string]$HostName,
        [int]$Port
    )

    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $asyncResult = $client.BeginConnect($HostName, $Port, $null, $null)
        if (-not $asyncResult.AsyncWaitHandle.WaitOne(2000)) {
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

Push-Location $backendDir
try {
    $env:APP_HOST = $AppHost
    $env:APP_PORT = $AppPort.ToString()
    $env:APP_STORAGE_BACKEND = "mysql"
    $env:MYSQL_URL = $MySqlUrl

    $mysqlHost = "127.0.0.1"
    $mysqlPort = 3306
    if ($MySqlUrl -match '^mysql://[^@]+@(?<host>[^:/]+)(:(?<port>\d+))?/') {
        $mysqlHost = $Matches.host
        if ($Matches.port) {
            $mysqlPort = [int]$Matches.port
        }
    }

    if (-not (Test-TcpEndpoint -HostName $mysqlHost -Port $mysqlPort)) {
        throw "无法连接到 MySQL：$mysqlHost`:$mysqlPort。请先启动 MySQL，再执行当前脚本。"
    }

    Write-Host "以 mysql 模式启动后端：$AppHost`:$AppPort" -ForegroundColor Cyan
    Write-Host "启动时会自动执行 backend/migrations 下尚未执行的迁移。" -ForegroundColor DarkCyan
    Write-Host "如需整链路验收，可在服务启动后执行：infra/scripts/run-mysql-acceptance.ps1" -ForegroundColor DarkCyan
    cargo.exe run
}
finally {
    Pop-Location
}
