$ErrorActionPreference = 'Stop'
$ChatRoot = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$HomeDir = $env:USERPROFILE
$WorkDir = $ChatRoot

function Import-AiSecrets {
    $secrets = Join-Path $ChatRoot 'ai-secrets.ps1'
    if (Test-Path -LiteralPath $secrets) {
        . $secrets
    }
}

function Apply-AllKeys {
    Import-AiSecrets
    if ($script:XAI_API_KEY) { $env:XAI_API_KEY = $script:XAI_API_KEY }
    if ($script:DEEPSEEK_API_KEY -and $script:DEEPSEEK_API_KEY -ne 'sk-...') {
        $env:DEEPSEEK_API_KEY = $script:DEEPSEEK_API_KEY
    }
    if ($script:GEMINI_API_KEY -and $script:GEMINI_API_KEY -notmatch '^\.\.\.|^AIza\.\.\.') {
        $env:GEMINI_API_KEY = $script:GEMINI_API_KEY
        $env:GOOGLE_API_KEY = $script:GEMINI_API_KEY
    }
    if ($script:GROQ_API_KEY -and $script:GROQ_API_KEY -notmatch '^\.\.\.|^gsk\.\.\.') {
        $env:GROQ_API_KEY = $script:GROQ_API_KEY
    }
    if ($script:OPENROUTER_API_KEY) { $env:OPENROUTER_API_KEY = $script:OPENROUTER_API_KEY }
    if ($script:HF_TOKEN) { $env:HF_TOKEN = $script:HF_TOKEN }
}

function Set-DeepSeekEnv {
    param([string]$ApiKey)
    if (-not $ApiKey) { return $false }
    $env:ANTHROPIC_BASE_URL = 'https://api.deepseek.com/anthropic'
    $env:ANTHROPIC_AUTH_TOKEN = $ApiKey
    $env:ANTHROPIC_MODEL = 'deepseek-v4-pro[1m]'
    $env:ANTHROPIC_DEFAULT_OPUS_MODEL = 'deepseek-v4-pro[1m]'
    $env:ANTHROPIC_DEFAULT_SONNET_MODEL = 'deepseek-v4-pro[1m]'
    $env:ANTHROPIC_DEFAULT_HAIKU_MODEL = 'deepseek-v4-flash'
    $env:CLAUDE_CODE_SUBAGENT_MODEL = 'deepseek-v4-flash'
    $env:CLAUDE_CODE_EFFORT_LEVEL = 'max'
    $env:DEEPSEEK_API_KEY = $ApiKey
    Remove-Item Env:ANTHROPIC_API_KEY -ErrorAction SilentlyContinue
    return $true
}

function Ensure-PathEntry([string]$Dir) {
    if (-not $Dir -or -not (Test-Path -LiteralPath $Dir)) { return }
    if ($env:Path -notlike "*$Dir*") {
        $env:Path = "$Dir;$env:Path"
    }
}

function Set-ToolWorkDirectory {
    param([string]$ToolId)
    $wf = Join-Path $ChatRoot '_work-folders.ps1'
    if (Test-Path -LiteralPath $wf) {
        . $wf
        $dir = Get-ToolWorkDirectory $ToolId
        if ($dir) {
            $script:WorkDir = $dir
            Set-Location -LiteralPath $dir
            return
        }
    }
    Set-WorkDirectory
}

function Show-ChatBanner([string]$Name, [string]$Color = 'Cyan', [string]$Tip = '', [string]$ToolId = '') {
    Write-Host ''
    Write-Host "  $Name" -ForegroundColor $Color
    Write-Host "  work:   $WorkDir" -ForegroundColor DarkGray
    if ($ToolId -eq 'codex') {
        $out = Join-Path $env:USERPROFILE 'Documents\Codex'
        Write-Host "  codex saves builds: $out" -ForegroundColor DarkYellow
        Write-Host "  shortcut: AI Work - Codex.lnk on desktop" -ForegroundColor DarkGray
    }
    if ($Tip) { Write-Host "  tip: $Tip" -ForegroundColor DarkYellow }
    Write-Host ''
}

function Require-Command([string]$Name, [string]$Hint) {
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $cmd) {
        Write-Host "  missing: $Name" -ForegroundColor Red
        Write-Host "  $Hint" -ForegroundColor Yellow
        Read-Host '  Enter to close'
        return $false
    }
    return $true
}

function Set-WorkDirectory {
    Set-Location -LiteralPath $WorkDir
}