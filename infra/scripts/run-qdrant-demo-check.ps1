[CmdletBinding()]
param(
    [string]$QdrantUrl = "http://127.0.0.1:6333",
    [string]$QdrantApiKey = "",
    [string]$CollectionName = "zhishu_segments_demo"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Parse-QdrantEndpoint {
    param([string]$Url)

    $trimmed = $Url.Trim()
    $defaultPort = if ($trimmed.StartsWith("https://")) { 443 } else { 6333 }
    $withoutScheme = $trimmed.Replace("http://", "")
    $withoutScheme = $withoutScheme.Replace("https://", "")
    $hostPort = $withoutScheme.Split('/')[0]
    $parts = $hostPort.Split(':')
    $endpointHost = if ($parts[0]) { $parts[0] } else { "127.0.0.1" }
    $endpointPort = if ($parts.Length -gt 1 -and $parts[1]) { [int]$parts[1] } else { $defaultPort }

    return @{
        Host = $endpointHost
        Port = $endpointPort
    }
}

if (-not $QdrantApiKey) {
    $QdrantApiKey = $env:QDRANT_API_KEY
}

Write-Host "检查 Qdrant demo collection：" -ForegroundColor Cyan
Write-Host "  QdrantUrl = $QdrantUrl" -ForegroundColor DarkCyan
Write-Host "  Collection = $CollectionName" -ForegroundColor DarkCyan
if ($QdrantApiKey) {
    Write-Host "  Auth = api-key header" -ForegroundColor DarkCyan
}

$headers = @{}
if ($QdrantApiKey) {
    $headers["api-key"] = $QdrantApiKey
}

try {
    $info = Invoke-RestMethod -Method Get -Uri "$QdrantUrl/collections/$CollectionName" -Headers $headers
}
catch {
    throw "Qdrant demo collection 检查失败：$($_.Exception.Message)"
}

Write-Host ""
Write-Host "Qdrant demo 检查通过：" -ForegroundColor Green
if ($info.result) {
    Write-Host ("- status = {0}" -f $info.result.status)
    if ($info.result.points_count -ne $null) {
        Write-Host ("- points.count = {0}" -f $info.result.points_count)
    }
    if ($info.result.config.params.vectors.size -ne $null) {
        Write-Host ("- vector.size = {0}" -f $info.result.config.params.vectors.size)
    }
    if ($info.result.config.params.vectors.distance) {
        Write-Host ("- vector.distance = {0}" -f $info.result.config.params.vectors.distance)
    }
}

Write-Host ""
Write-Host "说明：该检查仅验证 demo collection 与 points 是否存在，不代表真实 Embedding 检索质量。" -ForegroundColor Yellow
