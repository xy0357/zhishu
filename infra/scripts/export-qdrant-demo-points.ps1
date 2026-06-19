[CmdletBinding()]
param(
    [string]$BaseUrl = "http://127.0.0.1:8080/api",
    [string]$AdminUsername = "admin",
    [string]$AdminPassword = "Admin@123456",
    [string]$OutputPath = "artifacts\qdrant-demo-points.json",
    [int]$VectorSize = 8
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

function Parse-ApiEndpoint {
    param([string]$Url)

    $trimmed = $Url.Trim()
    $defaultPort = if ($trimmed.StartsWith("https://")) { 443 } else { 80 }
    if ($trimmed.StartsWith("http://")) {
        $defaultPort = 80
    }
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

function Invoke-ZhishuApi {
    param(
        [ValidateSet("GET", "POST")]
        [string]$Method,
        [string]$Path,
        [hashtable]$Headers,
        [object]$Body
    )

    $params = @{
        Method      = $Method
        Uri         = "$BaseUrl$Path"
        Headers     = $Headers
        ContentType = "application/json; charset=utf-8"
    }

    if ($null -ne $Body) {
        $params.Body = ($Body | ConvertTo-Json -Depth 8)
    }

    Invoke-RestMethod @params
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

function Sanitize-JsonText {
    param([string]$Text)

    if ($null -eq $Text) {
        return ""
    }

    $builder = New-Object System.Text.StringBuilder
    foreach ($char in $Text.ToCharArray()) {
        $code = [int][char]$char

        if ($code -eq 9 -or $code -eq 10 -or $code -eq 13) {
            [void]$builder.Append(' ')
            continue
        }

        if ($code -ge 32 -and $code -le 126) {
            [void]$builder.Append($char)
            continue
        }

        if (($code -ge 0 -and $code -le 31) -or
            ($code -ge 127 -and $code -le 159) -or
            ($code -ge 55296 -and $code -le 57343)) {
            continue
        }

        [void]$builder.Append('?')
    }

    $safe = $builder.ToString()
    $safe = [System.Text.RegularExpressions.Regex]::Replace($safe, '\s+', ' ').Trim()
    return $safe
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$outputFile = Join-Path $repoRoot $OutputPath
$outputDir = Split-Path -Parent $outputFile
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

Write-Host "导出 Qdrant demo points：" -ForegroundColor Cyan
Write-Host "  BaseUrl = $BaseUrl" -ForegroundColor DarkCyan
Write-Host "  Output = $outputFile" -ForegroundColor DarkCyan

$apiEndpoint = Parse-ApiEndpoint -Url $BaseUrl
if (-not (Test-TcpEndpoint -HostName $apiEndpoint.Host -Port $apiEndpoint.Port)) {
    throw "后端当前不可达：$($apiEndpoint.Host):$($apiEndpoint.Port)。请先启动 start-mysql-backend.ps1，再执行当前脚本。"
}

$login = Invoke-ZhishuApi -Method POST -Path "/auth/login" -Headers @{} -Body @{
    username = $AdminUsername
    password = $AdminPassword
}
$token = $login.data.access_token
$authHeaders = @{ Authorization = "Bearer $token" }

$documents = Invoke-ZhishuApi -Method GET -Path "/documents" -Headers $authHeaders -Body $null
$points = New-Object System.Collections.Generic.List[object]

foreach ($document in $documents.data) {
    $segments = Invoke-ZhishuApi -Method GET -Path "/documents/$($document.document_id)/segments" -Headers $authHeaders -Body $null
    foreach ($segment in $segments.data) {
        $safeChunkText = Sanitize-JsonText -Text ([string]$segment.chunk_text)
        $vector = New-DemoVector -Text $safeChunkText -Size $VectorSize
        $points.Add([PSCustomObject]@{
            id      = [int64]$segment.segment_id
            vector  = $vector
            payload = [PSCustomObject]@{
                segment_id        = $segment.segment_id
                document_id       = $segment.document_id
                version_id        = $segment.version_id
                chunk_order       = $segment.chunk_order
                chunk_text        = $safeChunkText
                token_count       = $segment.token_count
                embedding_status  = $segment.embedding_status
                source            = "zhishu-demo-hash"
                vector_mode       = "demo_hash_v1"
            }
        }) | Out-Null
    }
}

$payload = [PSCustomObject]@{
    generated_at = (Get-Date).ToString("s")
    vector_size  = $VectorSize
    distance     = "Cosine"
    points       = $points
}

$payload | ConvertTo-Json -Depth 8 | Set-Content -Path $outputFile -Encoding UTF8

Write-Host ""
Write-Host "导出完成：" -ForegroundColor Green
Write-Host ("- documents.count = {0}" -f $documents.data.Count)
Write-Host ("- points.count = {0}" -f $points.Count)
Write-Host ("- vector_mode = demo_hash_v1") -ForegroundColor DarkCyan
Write-Host ""
Write-Host "说明：当前导出的是 demo hash 向量，仅用于打通 Qdrant 同步与演示链路，不代表真实 Embedding。" -ForegroundColor Yellow
