[CmdletBinding()]
param(
    [string]$BaseUrl = "http://127.0.0.1:8080/api",
    [string]$AdminUsername = "admin",
    [string]$AdminPassword = "Admin@123456"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Invoke-WithRetry {
    param(
        [scriptblock]$Action,
        [int]$MaxAttempts = 3,
        [int]$DelayMilliseconds = 400
    )

    $lastError = $null
    for ($attempt = 1; $attempt -le $MaxAttempts; $attempt++) {
        try {
            return & $Action
        }
        catch {
            $lastError = $_
            if ($attempt -lt $MaxAttempts) {
                Start-Sleep -Milliseconds $DelayMilliseconds
            }
        }
    }

    throw $lastError
}

function Invoke-ZhishuApi {
    param(
        [ValidateSet("GET", "POST", "PUT", "DELETE")]
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

    Invoke-WithRetry -Action { Invoke-RestMethod @params }
}

Write-Host "开始执行 MySQL 模式验收：" -ForegroundColor Cyan
Write-Host "  BaseUrl = $BaseUrl" -ForegroundColor DarkCyan

$health = Invoke-ZhishuApi -Method GET -Path "/health" -Headers @{} -Body $null
if (-not $health.success) {
    throw "健康检查失败，无法继续执行验收。"
}

$login = Invoke-ZhishuApi -Method POST -Path "/auth/login" -Headers @{} -Body @{
    username = $AdminUsername
    password = $AdminPassword
}

if (-not $login.success) {
    throw "登录失败，无法继续执行验收。"
}

$token = $login.data.access_token
$authHeaders = @{
    Authorization = "Bearer $token"
}

$dashboard = Invoke-ZhishuApi -Method GET -Path "/dashboard" -Headers $authHeaders -Body $null
$documents = Invoke-ZhishuApi -Method GET -Path "/documents" -Headers $authHeaders -Body $null
$categories = Invoke-ZhishuApi -Method GET -Path "/categories" -Headers $authHeaders -Body $null
$tags = Invoke-ZhishuApi -Method GET -Path "/tags" -Headers $authHeaders -Body $null
$roles = Invoke-ZhishuApi -Method GET -Path "/roles" -Headers $authHeaders -Body $null
$users = Invoke-ZhishuApi -Method GET -Path "/users" -Headers $authHeaders -Body $null

$timestamp = Get-Date -Format "yyyyMMddHHmmss"
$document = Invoke-ZhishuApi -Method POST -Path "/documents" -Headers $authHeaders -Body @{
    title         = "MySQL验收文档-$timestamp"
    summary       = "用于验证 MySQL 实库链路。"
    content       = "1. 验证登录。`n2. 验证文档写入。`n3. 验证问答与行为留痕。"
    category_name = "实库验收"
    tags          = @("mysql", "验收")
    change_note   = "MySQL 验收创建"
}

$documentId = $document.data.document_id

$faq = Invoke-ZhishuApi -Method POST -Path "/documents/$documentId/faqs" -Headers $authHeaders -Body @{
    question = "这份验收文档是做什么的？"
    answer   = "用于验证 MySQL 模式下的文档、FAQ、问答和行为链路。"
}

$publish = Invoke-ZhishuApi -Method POST -Path "/documents/$documentId/publish" -Headers $authHeaders -Body $null
$read = Invoke-ZhishuApi -Method POST -Path "/documents/$documentId/read" -Headers $authHeaders -Body $null
$favorite = Invoke-ZhishuApi -Method POST -Path "/documents/$documentId/favorite" -Headers $authHeaders -Body $null
$qa = Invoke-ZhishuApi -Method POST -Path "/qa/ask" -Headers $authHeaders -Body @{
    question_text = "数据库权限应该怎么申请？"
}

$history = Invoke-ZhishuApi -Method GET -Path "/questions/history" -Headers $authHeaders -Body $null
$favorites = Invoke-ZhishuApi -Method GET -Path "/favorites" -Headers $authHeaders -Body $null
$recentReads = Invoke-ZhishuApi -Method GET -Path "/read-records/recent" -Headers $authHeaders -Body $null
$agentRuns = Invoke-ZhishuApi -Method GET -Path "/agent-runs" -Headers $authHeaders -Body $null

Write-Host ""
Write-Host "MySQL 验收通过，关键结果如下：" -ForegroundColor Green
Write-Host ("- health.storage_backend = {0}" -f $health.data.storage_backend)
Write-Host ("- dashboard.total_documents = {0}" -f $dashboard.data.total_documents)
Write-Host ("- documents.count = {0}" -f $documents.data.Count)
Write-Host ("- categories.count = {0}" -f $categories.data.Count)
Write-Host ("- tags.count = {0}" -f $tags.data.Count)
Write-Host ("- roles.count = {0}" -f $roles.data.Count)
Write-Host ("- users.count = {0}" -f $users.data.Count)
Write-Host ("- created_document_id = {0}" -f $documentId)
Write-Host ("- created_faq_id = {0}" -f $faq.data.faq_id)
Write-Host ("- publish_status = {0}" -f $publish.data.status)
Write-Host ("- read_record_id = {0}" -f $read.data.read_id)
Write-Host ("- is_favorite = {0}" -f $favorite.data.is_favorite)
Write-Host ("- answer_id = {0}" -f $qa.data.answer_id)
Write-Host ("- citations.count = {0}" -f $qa.data.citations.Count)
Write-Host ("- question_history.count = {0}" -f $history.data.Count)
Write-Host ("- favorites.count = {0}" -f $favorites.data.Count)
Write-Host ("- recent_reads.count = {0}" -f $recentReads.data.Count)
Write-Host ("- agent_runs.count = {0}" -f $agentRuns.data.Count)

Write-Host ""
Write-Host "说明：该脚本默认要求后端已通过 start-mysql-backend.ps1 以 mysql 模式启动。" -ForegroundColor Yellow
