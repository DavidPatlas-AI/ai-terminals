$ErrorActionPreference = 'Continue'
$ChatRoot = $PSScriptRoot
. (Join-Path $ChatRoot '_status.ps1')
. (Join-Path $ChatRoot '_conversations.ps1')

$jsonPath = Join-Path $ChatRoot 'status.json'
$payload = $null
if (Test-Path -LiteralPath $jsonPath) {
    try { $payload = Get-Content -LiteralPath $jsonPath -Raw -Encoding UTF8 | ConvertFrom-Json } catch {}
}
if (-not $payload) {
    Write-Host 'No status.json — run check-all.bat first' -ForegroundColor Yellow
    exit 1
}

$convData = Get-ConversationDashboard
$checked = Get-Date -Format 'yyyy-MM-dd HH:mm'
$out = @{
    checkedAt = $checked
    folder = $payload.folder
    models = $payload.models
    catalog = $payload.catalog
    extra = $payload.extra
    freeOptions = $payload.freeOptions
    freeDetect = $payload.freeDetect
    workingCount = $payload.workingCount
    totalCount = $payload.totalCount
    conversations = $convData.conversations
    tokenSummary = $convData.summary
}
[System.IO.File]::WriteAllText($jsonPath, ($out | ConvertTo-Json -Depth 8), [System.Text.UTF8Encoding]::new($false))
Write-DashboardHtml -ChatRoot $ChatRoot -Payload $out
Write-Host "Updated conversations: $($convData.conversations.Count) | Active: $($convData.summary.activeCount)" -ForegroundColor Green
Write-Host "Open: open-dashboard.bat" -ForegroundColor Cyan