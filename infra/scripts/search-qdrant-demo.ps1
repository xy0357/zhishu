[CmdletBinding()]
param(
    [string]$QdrantUrl = "http://127.0.0.1:6333",
    [string]$QdrantApiKey = "",
    [string]$CollectionName = "zhishu_segments_demo",
    [string]$QuestionText = "如何申请数据库权限？",
    [int]$VectorSize = 8,
    [int]$Limit = 3
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

function New-DemoVector {
    param(
        [string]$Text,
        [int]$Size
    )

    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    if ($bytes.Length -eq 0) {
        $bytes = [byte[]](0)
    }

    $vector = New-Object System.Collections.Generic.List[double]
    for ($index = 0; $index -lt $Size; $index++) {
        $sum = 0
        for ($offset = $index; $offset -lt $bytes.Length; $offset += $Size) {
            $sum += $bytes[$offset]
        }
        $value = [Math]::Round(($sum % 256) / 255.0, 6)
        $vector.Add($value)
    }

    return $vector
}

$queryVector = New-DemoVector -Text $QuestionText -Size $VectorSize
$body = @{
    query         = $queryVector
    limit         = $Limit
    with_payload  = $true
    with_vector   = $false
} | ConvertTo-Json -Depth 8

Write-Host "查询 Qdrant demo collection：" -ForegroundColor Cyan
Write-Host "  QdrantUrl = $QdrantUrl" -ForegroundColor DarkCyan
Write-Host "  Collection = $CollectionName" -ForegroundColor DarkCyan
Write-Host "  Question = $QuestionText" -ForegroundColor DarkCyan
if ($QdrantApiKey) {
    Write-Host "  Auth = api-key header" -ForegroundColor DarkCyan
}

$headers = @{}
if ($QdrantApiKey) {
    $headers["api-key"] = $QdrantApiKey
}

try {
    $response = Invoke-RestMethod -Method Post -Uri "$QdrantUrl/collections/$CollectionName/points/query" -Headers $headers -ContentType "application/json" -Body $body
}
catch {
    throw "Qdrant demo 查询失败：$($_.Exception.Message)"
}

Write-Host ""
Write-Host "Qdrant demo 查询结果：" -ForegroundColor Green
$results = @($response.result.points)
Write-Host ("- results.count = {0}" -f $results.Count)

foreach ($item in $results) {
    Write-Host ""
    Write-Host ("- point.id = {0}" -f $item.id) -ForegroundColor DarkCyan
    Write-Host ("  score = {0}" -f $item.score)
    if ($item.payload.segment_id -ne $null) {
        Write-Host ("  segment_id = {0}" -f $item.payload.segment_id)
    }
    if ($item.payload.document_id -ne $null) {
        Write-Host ("  document_id = {0}" -f $item.payload.document_id)
    }
    if ($item.payload.chunk_text) {
        Write-Host ("  chunk_text = {0}" -f $item.payload.chunk_text)
    }
}

Write-Host ""
Write-Host "说明：该脚本使用 `demo_hash_v1` 向量查询 Qdrant，只用于演示 Query API 流程，不代表真实语义检索质量。" -ForegroundColor Yellow
