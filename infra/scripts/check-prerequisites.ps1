[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$checks = @(
    @{ Name = "cargo"; Command = "cargo.exe"; Args = @("--version") },
    @{ Name = "npm"; Command = "npm.cmd"; Args = @("--version") },
    @{ Name = "docker"; Command = "docker"; Args = @("--version") },
    @{ Name = "mysql"; Command = "mysql"; Args = @("--version") }
)

$results = foreach ($check in $checks) {
    $command = Get-Command $check.Command -ErrorAction SilentlyContinue
    if (-not $command) {
        [PSCustomObject]@{
            Name = $check.Name
            Status = "missing"
            Detail = "未安装或未加入 PATH"
        }
        continue
    }

    $detail = try {
        & $check.Command @($check.Args) 2>$null | Select-Object -First 1
    } catch {
        "已安装，但版本读取失败"
    }

    [PSCustomObject]@{
        Name = $check.Name
        Status = "ok"
        Detail = $detail
    }
}

$results | Format-Table -AutoSize

$missing = $results | Where-Object { $_.Status -ne "ok" }
if ($missing) {
    Write-Host ""
    Write-Host "说明：缺失项不会影响 file 模式演示，但会影响真实依赖链路。" -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "环境检查通过，可以继续启动知枢工程。" -ForegroundColor Green
