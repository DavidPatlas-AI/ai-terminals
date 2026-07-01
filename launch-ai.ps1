param(
    [Alias('Name', 'Tool')]
    [string]$Model,
    [switch]$List
)

$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot '_common.ps1')
. (Join-Path $PSScriptRoot '_models.ps1')

function As-Array {
    param($Value)
    if ($null -eq $Value) { return @() }
    if ($Value -is [System.Array]) { return $Value }
    return @($Value)
}

function Get-LaunchProfileMap {
    $map = @{}
    foreach ($profile in (As-Array $script:AiLaunchProfiles)) {
        if (-not $profile.Id) { continue }
        $map[$profile.Id.ToLowerInvariant()] = $profile
        foreach ($alias in (As-Array $profile.Aliases)) {
            if ($alias) { $map[$alias.ToLowerInvariant()] = $profile }
        }
    }
    return $map
}

function Show-AvailableModels {
    Write-Host ''
    Write-Host '  AI universal launcher' -ForegroundColor Cyan
    Write-Host ''
    Write-Host '  Usage:' -ForegroundColor DarkGray
    Write-Host '    AI.bat grok'
    Write-Host '    powershell -File launch-ai.ps1 -Model codex'
    Write-Host ''
    Write-Host '  Models:' -ForegroundColor DarkGray
    foreach ($profile in (As-Array $script:AiLaunchProfiles)) {
        $aliases = (As-Array $profile.Aliases) -join ', '
        $suffix = if ($aliases) { "  ($aliases)" } else { '' }
        Write-Host ("    {0,-11} {1}{2}" -f $profile.Id, $profile.Title, $suffix)
    }
    Write-Host ''
}

function Add-ProfilePathHints {
    param($Profile)
    foreach ($hint in (As-Array $Profile.PathHints)) {
        if (-not $hint) { continue }
        Ensure-PathEntry ([Environment]::ExpandEnvironmentVariables($hint))
    }
    if ($Profile.AddCodexPath) {
        $codexDir = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex\bin'
        if (Test-Path -LiteralPath $codexDir) {
            $codexExe = Get-ChildItem -LiteralPath $codexDir -Recurse -Filter 'codex.exe' -ErrorAction SilentlyContinue |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if ($codexExe) { Ensure-PathEntry $codexExe.DirectoryName }
        }
    }
}

function Enter-ProfileWorkDir {
    param($Profile)
    if ($Profile.WorkId) {
        Set-ToolWorkDirectory $Profile.WorkId
    } else {
        Set-WorkDirectory
    }
}

function Show-ProfileBanner {
    param($Profile)
    $bannerArgs = @{}
    if ($Profile.ToolId) { $bannerArgs.ToolId = $Profile.ToolId }
    Show-ChatBanner $Profile.Title $Profile.Color $Profile.Tip @bannerArgs
}

function Test-AnyEnvironment {
    param([string[]]$Names)
    foreach ($name in (As-Array $Names)) {
        if ($name -and [Environment]::GetEnvironmentVariable($name)) { return $true }
    }
    return $false
}

function Show-MissingEnvironment {
    param($Profile)
    Write-Host ''
    foreach ($line in (As-Array $Profile.MissingEnvLines)) {
        $color = if ($line -match '^Missing|^Need') { 'Red' } else { 'Yellow' }
        Write-Host "  $line" -ForegroundColor $color
    }
    Write-Host ''
    Read-Host '  Press Enter to close'
}

function Set-ProfileClipboard {
    param($Profile)
    if (-not $Profile.Clipboard) { return }
    try { Set-Clipboard -Value $Profile.Clipboard } catch {}
}

function Invoke-CommandProfile {
    param($Profile)
    Apply-AllKeys
    Add-ProfilePathHints $Profile
    Enter-ProfileWorkDir $Profile
    Show-ProfileBanner $Profile

    if ($Profile.RequireAnyEnv -and -not (Test-AnyEnvironment (As-Array $Profile.RequireAnyEnv))) {
        Show-MissingEnvironment $Profile
        return
    }

    if (-not (Require-Command $Profile.Command $Profile.MissingHint)) { return }
    Set-ProfileClipboard $Profile

    $args = @(As-Array $Profile.Args)
    if ($args.Count -gt 0) {
        & $Profile.Command @args
    } else {
        & $Profile.Command
    }
}

function Invoke-DeepSeekProfile {
    param($Profile)
    Import-AiSecrets
    Enter-ProfileWorkDir $Profile
    Show-ProfileBanner $Profile

    $key = $script:DEEPSEEK_API_KEY
    if (-not $key -or $key -eq 'sk-...') {
        Write-Host '  Missing DeepSeek API key!' -ForegroundColor Red
        Write-Host ''
        Write-Host '  1. Get key: https://platform.deepseek.com/api_keys' -ForegroundColor Yellow
        Write-Host '  2. Copy ai-secrets.example.ps1 to ai-secrets.ps1' -ForegroundColor Yellow
        Write-Host '  3. Paste key into $script:DEEPSEEK_API_KEY' -ForegroundColor Yellow
        Write-Host ''
        Read-Host '  Press Enter to close'
        return
    }

    Set-DeepSeekEnv $key | Out-Null
    Remove-Item Env:ANTHROPIC_API_KEY -ErrorAction SilentlyContinue

    $testBody = @{
        model = 'deepseek-v4-flash'
        messages = @(@{ role = 'user'; content = 'hi' })
        max_tokens = 3
    } | ConvertTo-Json -Depth 4 -Compress

    try {
        $null = Invoke-RestMethod -Uri 'https://api.deepseek.com/chat/completions' `
            -Method Post `
            -Headers @{ Authorization = "Bearer $key"; 'Content-Type' = 'application/json' } `
            -Body $testBody -TimeoutSec 30
        Write-Host '  DeepSeek API connected' -ForegroundColor Green
    } catch {
        if ($_.Exception.Message -match '402' -or $_.ErrorDetails.Message -match '402|Insufficient') {
            Write-Host ''
            Write-Host '  DeepSeek: Insufficient Balance' -ForegroundColor Red
            Write-Host '  Key is valid but account has no credits.' -ForegroundColor Yellow
            Write-Host '  Top up: https://platform.deepseek.com/top_up' -ForegroundColor Yellow
            Write-Host ''
            Read-Host '  Press Enter to close'
            return
        }
        Write-Host "  DeepSeek API warning: $($_.Exception.Message)" -ForegroundColor Yellow
    }

    Write-Host '  Model: deepseek-v4-pro' -ForegroundColor DarkGray
    Write-Host ''
    if (-not (Require-Command $Profile.Command $Profile.MissingHint)) { return }
    & $Profile.Command
}

function Invoke-OllamaProfile {
    param($Profile)
    Apply-AllKeys
    Enter-ProfileWorkDir $Profile
    Show-ProfileBanner $Profile

    if (-not (Require-Command $Profile.Command $Profile.MissingHint)) { return }
    $models = & $Profile.Command list 2>&1 | Out-String
    if ($models -notmatch 'llama|qwen|gemma|mistral|phi') {
        Write-Host '  No models yet. Pulling llama3.2 (small, fast)...' -ForegroundColor Yellow
        Write-Host '  This downloads about 2GB once.' -ForegroundColor DarkGray
        & $Profile.Command pull llama3.2
    }

    Write-Host ''
    Write-Host '  Chat: ollama run llama3.2' -ForegroundColor Cyan
    Write-Host '  Or use Crush/Aider with Ollama backend' -ForegroundColor DarkGray
    Write-Host ''
    Set-ProfileClipboard $Profile
    & $Profile.Command run llama3.2
}

function Invoke-DelegateProfile {
    param($Profile)
    $target = Join-Path $PSScriptRoot $Profile.Script
    if (-not (Test-Path -LiteralPath $target)) {
        Write-Host "  Missing delegated script: $($Profile.Script)" -ForegroundColor Red
        Read-Host '  Press Enter to close'
        return
    }
    & $target
}

$profiles = Get-LaunchProfileMap
if ($List -or -not $Model) {
    Show-AvailableModels
    return
}

$key = $Model.ToLowerInvariant()
if (-not $profiles.ContainsKey($key)) {
    Write-Host ''
    Write-Host "  Unknown model: $Model" -ForegroundColor Red
    Show-AvailableModels
    return
}

$profile = $profiles[$key]
switch ($profile.Kind) {
    'deepseek' { Invoke-DeepSeekProfile $profile; break }
    'ollama' { Invoke-OllamaProfile $profile; break }
    'delegate' { Invoke-DelegateProfile $profile; break }
    default { Invoke-CommandProfile $profile; break }
}
