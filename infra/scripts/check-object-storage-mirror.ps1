[CmdletBinding()]
param(
    [string]$BaseUrl = "http://127.0.0.1:8080/api",
    [string]$AdminUsername = "admin",
    [string]$AdminPassword = "Admin@123456",
    [string]$ObjectStorageDir = "backend\data\object-storage"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

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

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$storageRoot = Join-Path $repoRoot $ObjectStorageDir

Write-Host "检查对象存储镜像：" -ForegroundColor Cyan
Write-Host "  BaseUrl = $BaseUrl" -ForegroundColor DarkCyan
Write-Host "  StorageRoot = $storageRoot" -ForegroundColor DarkCyan

$login = Invoke-ZhishuApi -Method POST -Path "/auth/login" -Headers @{} -Body @{
    username = $AdminUsername
    password = $AdminPassword
}
$token = $login.data.access_token
$authHeaders = @{ Authorization = "Bearer $token" }
$files = Invoke-ZhishuApi -Method GET -Path "/document-files" -Headers $authHeaders -Body $null

$missing = New-Object System.Collections.Generic.List[string]
$matched = 0

foreach ($file in $files.data) {
    $parts = $file.object_key -split '/'
    $objectPath = $storageRoot
    foreach ($part in $parts) {
        $objectPath = Join-Path $objectPath $part
    }

    if (Test-Path $objectPath) {
        $matched += 1
    }
    else {
        $missing.Add($file.object_key) | Out-Null
    }
}

Write-Host ""
Write-Host "对象存储镜像检查结果：" -ForegroundColor Green
Write-Host ("- document_files.count = {0}" -f $files.data.Count)
Write-Host ("- matched.count = {0}" -f $matched)
Write-Host ("- missing.count = {0}" -f $missing.Count)

if ($missing.Count -gt 0) {
    Write-Host ""
    Write-Host "缺失 object_key：" -ForegroundColor Yellow
    $missing | ForEach-Object { Write-Host ("- {0}" -f $_) -ForegroundColor Yellow }
    exit 1
}

Write-Host ""
Write-Host "说明：该脚本验证 document_files 与 OBJECT_STORAGE_DIR 本地对象镜像是否一一对应。" -ForegroundColor Yellow
