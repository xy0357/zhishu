[CmdletBinding()]
param(
    [string]$QdrantUrl = "http://127.0.0.1:6333",
    [string]$QdrantApiKey = "",
    [string]$CollectionName = "zhishu_segments_demo",
    [string]$InputPath = "artifacts\qdrant-demo-points.json"
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

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$inputFile = Join-Path $repoRoot $InputPath
if (-not $QdrantApiKey) {
    $QdrantApiKey = $env:QDRANT_API_KEY
}

if (-not (Test-Path $inputFile)) {
    throw "找不到输入文件：$inputFile。请先执行 infra/scripts/export-qdrant-demo-points.ps1"
}

$payload = Get-Content -Path $inputFile -Encoding UTF8 -Raw | ConvertFrom-Json

$collectionBody = @{
    vectors = @{
        size     = [int]$payload.vector_size
        distance = $payload.distance
    }
} | ConvertTo-Json -Depth 8

$pointsBody = @{
    points = @($payload.points)
} | ConvertTo-Json -Depth 16

Write-Host "同步 demo points 到 Qdrant：" -ForegroundColor Cyan
Write-Host "  QdrantUrl = $QdrantUrl" -ForegroundColor DarkCyan
Write-Host "  Collection = $CollectionName" -ForegroundColor DarkCyan
Write-Host "  Input = $inputFile" -ForegroundColor DarkCyan
if ($QdrantApiKey) {
    Write-Host "  Auth = api-key header" -ForegroundColor DarkCyan
}

$headers = @{}
if ($QdrantApiKey) {
    $headers["api-key"] = $QdrantApiKey
}

try {
    Invoke-RestMethod -Method Put -Uri "$QdrantUrl/collections/$CollectionName" -Headers $headers -ContentType "application/json" -Body $collectionBody | Out-Null
}
catch {
    $response = $_.Exception.Response
    if ($response -and [int]$response.StatusCode -eq 409) {
        Write-Host "  Collection 已存在，继续执行 points upsert。" -ForegroundColor Yellow
    }
    elseif ($response) {
        $reader = New-Object System.IO.StreamReader($response.GetResponseStream())
        $errorBody = $reader.ReadToEnd()
        throw "Qdrant collection 创建失败：$errorBody"
    }
    else {
        throw
    }
}
try {
    Invoke-RestMethod -Method Put -Uri "$QdrantUrl/collections/$CollectionName/points?wait=true" -Headers $headers -ContentType "application/json" -Body $pointsBody | Out-Null
}
catch {
    $response = $_.Exception.Response
    if ($response) {
        $reader = New-Object System.IO.StreamReader($response.GetResponseStream())
        $errorBody = $reader.ReadToEnd()
        throw "Qdrant points upsert 失败：$errorBody"
    }
    throw "Qdrant points upsert 失败：$($_.Exception.Message)"
}
$collectionInfo = Invoke-RestMethod -Method Get -Uri "$QdrantUrl/collections/$CollectionName" -Headers $headers

Write-Host ""
Write-Host "Qdrant demo 同步完成：" -ForegroundColor Green
Write-Host ("- collection = {0}" -f $CollectionName)
Write-Host ("- points.count = {0}" -f $payload.points.Count)
if ($collectionInfo.result) {
    Write-Host ("- status = {0}" -f $collectionInfo.result.status)
}
Write-Host ""
Write-Host "说明：该脚本使用的是 demo hash 向量，只用于验证 collection 创建与 points upsert 链路。" -ForegroundColor Yellow
