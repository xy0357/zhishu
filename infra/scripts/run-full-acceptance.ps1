[CmdletBinding()]
param(
    [string]$BaseUrl = "http://127.0.0.1:8080/api",
    [string]$AdminUsername = "admin",
    [string]$AdminPassword = "Admin@123456"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

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

    Invoke-RestMethod @params
}

Write-Host "开始执行全量验收：" -ForegroundColor Cyan
Write-Host "  BaseUrl = $BaseUrl" -ForegroundColor DarkCyan

$health = Invoke-ZhishuApi -Method GET -Path "/health" -Headers @{} -Body $null
$login = Invoke-ZhishuApi -Method POST -Path "/auth/login" -Headers @{} -Body @{
    username = $AdminUsername
    password = $AdminPassword
}
$token = $login.data.access_token
$authHeaders = @{
    Authorization = "Bearer $token"
}

$refresh = Invoke-ZhishuApi -Method POST -Path "/auth/refresh" -Headers $authHeaders -Body $null
$dashboard = Invoke-ZhishuApi -Method GET -Path "/dashboard" -Headers $authHeaders -Body $null
$documentFiles = Invoke-ZhishuApi -Method GET -Path "/document-files" -Headers $authHeaders -Body $null

$timestamp = Get-Date -Format "yyyyMMddHHmmss"
$document = Invoke-ZhishuApi -Method POST -Path "/documents" -Headers $authHeaders -Body @{
    title         = "全量验收文档-$timestamp"
    summary       = "用于验证重建分段、问答引用与后台管理能力。"
    content       = "1. 创建验收文档。`n2. 发布并生成分段。`n3. 问答引用真实片段。"
    category_name = "实库验收"
    tags          = @("full", "acceptance")
    change_note   = "全量验收创建"
}
$documentId = $document.data.document_id

$publish = Invoke-ZhishuApi -Method POST -Path "/documents/$documentId/publish" -Headers $authHeaders -Body $null
$segments = Invoke-ZhishuApi -Method GET -Path "/documents/$documentId/segments" -Headers $authHeaders -Body $null
$reindex = Invoke-ZhishuApi -Method POST -Path "/documents/$documentId/reindex" -Headers $authHeaders -Body $null
$qa = Invoke-ZhishuApi -Method POST -Path "/qa/ask" -Headers $authHeaders -Body @{
    question_text = "全量验收文档说明了什么？"
}

$newUser = Invoke-ZhishuApi -Method POST -Path "/users" -Headers $authHeaders -Body @{
    username   = "acceptance_$timestamp"
    role_name  = "普通用户"
    department = "验收"
    email      = "acceptance_$timestamp@example.com"
    password   = "Acceptance@123456"
}
$newUserId = $newUser.data.user_id
$reset = Invoke-ZhishuApi -Method POST -Path "/users/$newUserId/reset-password" -Headers $authHeaders -Body @{
    password = "Acceptance@654321"
}
$delete = Invoke-ZhishuApi -Method DELETE -Path "/users/$newUserId" -Headers $authHeaders -Body $null

Write-Host ""
Write-Host "全量验收通过，关键结果如下：" -ForegroundColor Green
Write-Host ("- health.storage_backend = {0}" -f $health.data.storage_backend)
Write-Host ("- refresh.token_type = {0}" -f $refresh.data.token_type)
Write-Host ("- dashboard.total_documents = {0}" -f $dashboard.data.total_documents)
Write-Host ("- document_files.count = {0}" -f $documentFiles.data.Count)
Write-Host ("- created_document_id = {0}" -f $documentId)
Write-Host ("- publish_status = {0}" -f $publish.data.status)
Write-Host ("- segments.count = {0}" -f $segments.data.Count)
Write-Host ("- reindex_status = {0}" -f $reindex.data.status)
Write-Host ("- qa.answer_id = {0}" -f $qa.data.answer_id)
Write-Host ("- qa.first_segment_id = {0}" -f $qa.data.citations[0].segment_id)
Write-Host ("- reset_user_id = {0}" -f $reset.data.user_id)
Write-Host ("- delete.resource_key = {0}" -f $delete.data.resource_key)

Write-Host ""
Write-Host "说明：该脚本默认要求 MySQL 模式后端已启动，且管理员账号可登录。" -ForegroundColor Yellow
