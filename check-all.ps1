$ErrorActionPreference = 'Continue'
$ChatRoot = $PSScriptRoot
. (Join-Path $ChatRoot '_common.ps1')
. (Join-Path $ChatRoot '_models.ps1')
. (Join-Path $ChatRoot '_status.ps1')
. (Join-Path $ChatRoot '_free-options.ps1')
. (Join-Path $ChatRoot '_conversations.ps1')
Import-AiSecrets

Write-Host ''
Write-Host '  Checking all models...' -ForegroundColor Cyan
Write-Host ''

$models = @(
    Test-GrokStatus
    Test-ClaudeStatus
    Test-GeminiStatus -ApiKey $script:GEMINI_API_KEY
    Test-DeepSeekStatus -ApiKey $script:DEEPSEEK_API_KEY
    Test-CodexStatus
    Test-NotebookLMStatus
)

$checked = Get-Date -Format 'yyyy-MM-dd HH:mm'
$detect = Get-FreeOptionDetect
$working = Get-WorkingCount $models
$convData = Get-ConversationDashboard
$payload = @{
    checkedAt = $checked
    folder = $ChatRoot
    models = $models
    catalog = $script:AiModels
    extra = $script:ExtraModels
    freeOptions = $script:FreeOptions
    freeDetect = $detect
    workingCount = $working
    totalCount = $models.Count
    conversations = $convData.conversations
    tokenSummary = $convData.summary
}

Write-Host '=== AI Terminal Status ===' -ForegroundColor Cyan
Write-Host "Checked: $checked" -ForegroundColor DarkGray
Write-Host ''
$colors = @{ ok = 'Green'; wait = 'Yellow'; fail = 'Red'; key = 'Magenta' }
foreach ($m in $models) {
    $c = if ($colors.ContainsKey($m.status)) { $colors[$m.status] } else { 'White' }
    $tag = "[$($m.status.ToUpper())]".PadRight(8)
    Write-Host "$tag $($m.name) ($($m.provider))" -ForegroundColor $c
    if ($m.balance) { Write-Host "         Balance: $($m.balance)" -ForegroundColor DarkGray }
    if ($m.limitReset) { Write-Host "         Resets: $($m.limitReset)" -ForegroundColor DarkYellow }
    if ($m.detail) { Write-Host "         $($m.detail)" -ForegroundColor DarkGray }
}

$lines = @("AI Chats - last check: $checked", '')
foreach ($m in $models) {
    $extra = @()
    if ($m.balance) { $extra += "balance: $($m.balance)" }
    if ($m.limitReset) { $extra += "reset: $($m.limitReset)" }
    $suffix = if ($extra.Count) { " ; " + ($extra -join " ; ") } else { "" }
    $lines += "[$($m.status.ToUpper())] $($m.name) - $($m.detail)$suffix"
}
$statusPath = Join-Path $ChatRoot 'STATUS.txt'
[System.IO.File]::WriteAllText($statusPath, ($lines -join "`r`n"), [System.Text.UTF8Encoding]::new($false))

$jsonPath = Join-Path $ChatRoot 'status.json'
[System.IO.File]::WriteAllText($jsonPath, ($payload | ConvertTo-Json -Depth 6), [System.Text.UTF8Encoding]::new($false))
Write-DashboardHtml -ChatRoot $ChatRoot -Payload $payload

if ($working -eq 0) {
    Write-Host ''
    Write-Host '*** Nothing working? Run try-free.bat ***' -ForegroundColor Yellow
    Write-Host '    Gemini key (free): aistudio.google.com/apikey' -ForegroundColor DarkYellow
    Write-Host '    Ollama (100% free): ollama.com/download' -ForegroundColor DarkYellow
}
Write-Host ''
Write-Host 'Saved: STATUS.txt, status.json, dashboard.html' -ForegroundColor DarkGray
Write-Host 'Open: open-dashboard.bat | Free: try-free.bat' -ForegroundColor Cyan
Write-Host ''