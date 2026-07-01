function New-ModelStatus {
    param(
        [string]$Id, [string]$Name, [string]$Provider,
        [string]$Status = 'unknown', [string]$Installed = 'unknown',
        [string]$Balance = '', [string]$LimitReset = '',
        [string]$Detail = '', [string]$Auth = '', [string]$Tier = ''
    )
    [PSCustomObject]@{
        id = $Id; name = $Name; provider = $Provider
        status = $Status; installed = $Installed
        balance = $Balance; limitReset = $LimitReset
        detail = $Detail; auth = $Auth; tier = $Tier
        checkedAt = (Get-Date -Format 'yyyy-MM-dd HH:mm')
    }
}

function Parse-LimitReset([string]$Text) {
    if ($Text -match 'resets\s+(\d{1,2}(?::\d{2})?\s*(?:am|pm)?)\s*(?:\(([^)]+)\))?') {
        $when = $Matches[1].Trim()
        $tz = if ($Matches[2]) { $Matches[2] } else { '' }
        if ($tz) { return "$when ($tz)" }
        return $when
    }
    if ($Text -match 'try again at\s+([0-9:]+\s*[AP]M)') { return $Matches[1].Trim() }
    return ''
}

function Test-GrokStatus {
    $installed = [bool](Get-Command grok -EA SilentlyContinue)
    if (-not $installed) {
        return New-ModelStatus 'grok' 'Grok' 'xAI' 'fail' 'no' '' '' 'Not installed' 'login' 'xAI plan'
    }
    try {
        $env:Path += ";$env:USERPROFILE\.grok\bin"
        $out = grok -p 'Reply exactly: OK' --yolo 2>&1 | ForEach-Object { $_.ToString() } | Out-String
        if ($out -match '\bOK\b') {
            return New-ModelStatus 'grok' 'Grok' 'xAI' 'ok' 'yes' 'Active plan' '' 'API responded' 'login' 'xAI plan'
        }
        if ($out -match 'credit|balance|limit|quota') {
            $reset = Parse-LimitReset $out
            return New-ModelStatus 'grok' 'Grok' 'xAI' 'wait' 'yes' '' $reset ($out.Trim() -split "`n" | Select-Object -Last 1) 'login' 'xAI plan'
        }
        return New-ModelStatus 'grok' 'Grok' 'xAI' 'fail' 'yes' '' '' (($out.Trim() -split "`n" | Select-Object -Last 2) -join ' ') 'login' 'xAI plan'
    } catch {
        return New-ModelStatus 'grok' 'Grok' 'xAI' 'fail' 'yes' '' '' $_.Exception.Message 'login' 'xAI plan'
    }
}

function Test-ClaudeStatus {
    $installed = [bool](Get-Command claude -EA SilentlyContinue)
    if (-not $installed) {
        return New-ModelStatus 'claude' 'Claude' 'Anthropic' 'fail' 'no' '' '' 'Not installed' 'login' 'Claude plan'
    }
    try {
        Push-Location (Join-Path $env:USERPROFILE 'Desktop')
        $out = claude -p 'Reply exactly: OK' --output-format text 2>&1 | ForEach-Object { $_.ToString() } | Out-String
        if ($out -match '\bOK\b') {
            return New-ModelStatus 'claude' 'Claude' 'Anthropic' 'ok' 'yes' 'Session active' '' 'API responded' 'login' 'Claude plan'
        }
        if ($out -match 'session limit|usage limit') {
            $reset = Parse-LimitReset $out
            return New-ModelStatus 'claude' 'Claude' 'Anthropic' 'wait' 'yes' 'Session cap' $reset ($out.Trim() -split "`n" | Select-Object -Last 1) 'login' 'Claude plan'
        }
        return New-ModelStatus 'claude' 'Claude' 'Anthropic' 'fail' 'yes' '' '' (($out.Trim() -split "`n" | Select-Object -Last 2) -join ' ') 'login' 'Claude plan'
    } catch {
        return New-ModelStatus 'claude' 'Claude' 'Anthropic' 'fail' 'yes' '' '' $_.Exception.Message 'login' 'Claude plan'
    } finally { Pop-Location }
}

function Test-CodexStatus {
    $exe = (Get-ChildItem "$env:LOCALAPPDATA\OpenAI\Codex\bin" -Recurse -Filter codex.exe -EA SilentlyContinue | Select-Object -First 1)
    if (-not $exe) {
        return New-ModelStatus 'codex' 'Codex' 'OpenAI' 'fail' 'no' '' '' 'Not installed' 'login' 'ChatGPT plan'
    }
    try {
        $outFile = Join-Path $env:TEMP 'ai-codex-check.out'
        $errFile = Join-Path $env:TEMP 'ai-codex-check.err'
        Remove-Item $outFile, $errFile -EA SilentlyContinue
        $args = 'exec --skip-git-repo-check --sandbox read-only -c approval_policy="never" "Reply exactly: OK"'
        $null = Start-Process -FilePath $exe.FullName -ArgumentList $args `
            -WorkingDirectory (Join-Path $env:USERPROFILE 'Desktop') `
            -NoNewWindow -Wait -RedirectStandardOutput $outFile -RedirectStandardError $errFile
        $chunks = @()
        if (Test-Path $outFile) { $chunks += Get-Content $outFile -EA SilentlyContinue }
        if (Test-Path $errFile) { $chunks += Get-Content $errFile -EA SilentlyContinue }
        $out = ($chunks -join "`n")
        if ($out -match '\bOK\b') {
            return New-ModelStatus 'codex' 'Codex' 'OpenAI' 'ok' 'yes' 'Active' '' 'API responded' 'login' 'ChatGPT plan'
        }
        if ($out -match 'usage limit|hit your usage|try again at') {
            $reset = Parse-LimitReset $out
            $detail = ($out.Trim() -split "`n" | Where-Object { $_ -match 'usage limit|try again' } | Select-Object -Last 1)
            if (-not $detail) { $detail = 'Usage limit reached' }
            return New-ModelStatus 'codex' 'Codex' 'OpenAI' 'wait' 'yes' 'Usage cap' $reset $detail 'login' 'ChatGPT plan'
        }
        return New-ModelStatus 'codex' 'Codex' 'OpenAI' 'fail' 'yes' '' '' (($out.Trim() -split "`n" | Select-Object -Last 3) -join ' | ') 'login' 'ChatGPT plan'
    } catch {
        return New-ModelStatus 'codex' 'Codex' 'OpenAI' 'fail' 'yes' '' '' $_.Exception.Message 'login' 'ChatGPT plan'
    }
}

function Test-DeepSeekStatus {
    param([string]$ApiKey)
    if (-not $ApiKey -or $ApiKey -eq 'sk-...') {
        return New-ModelStatus 'deepseek' 'DeepSeek' 'DeepSeek' 'key' 'yes' '' '' 'Missing API key - run setup-keys.bat' 'apikey' 'Pay per use'
    }
    $balance = ''
    try {
        $bal = Invoke-RestMethod -Uri 'https://api.deepseek.com/user/balance' `
            -Headers @{ Authorization = "Bearer $ApiKey" } -TimeoutSec 20
        if ($bal.balance_infos -and $bal.balance_infos.Count -gt 0) {
            $b = $bal.balance_infos[0]
            $balance = "`$$($b.total_balance) $($b.currency)"
            if (-not $bal.is_available) { $balance += ' (empty)' }
        }
    } catch { $balance = '?' }

    try {
        $body = '{"model":"deepseek-v4-flash","messages":[{"role":"user","content":"hi"}],"max_tokens":3}'
        $null = Invoke-RestMethod -Uri 'https://api.deepseek.com/chat/completions' -Method Post `
            -Headers @{ Authorization = "Bearer $ApiKey"; 'Content-Type' = 'application/json' } -Body $body -TimeoutSec 30
        return New-ModelStatus 'deepseek' 'DeepSeek' 'DeepSeek' 'ok' 'yes' $balance '' 'API responded' 'apikey' 'Pay per use'
    } catch {
        if ($_.Exception.Message -match '402' -or $_.ErrorDetails.Message -match '402|Insufficient') {
            return New-ModelStatus 'deepseek' 'DeepSeek' 'DeepSeek' 'fail' 'yes' $balance '' 'Insufficient balance - top up' 'apikey' 'Pay per use'
        }
        return New-ModelStatus 'deepseek' 'DeepSeek' 'DeepSeek' 'fail' 'yes' $balance '' $_.Exception.Message 'apikey' 'Pay per use'
    }
}

function Test-GeminiStatus {
    param([string]$ApiKey)
    $installed = [bool](Get-Command gemini -EA SilentlyContinue)
    if (-not $installed) {
        return New-ModelStatus 'gemini' 'Gemini' 'Google' 'fail' 'no' '' '' 'Run: npm install -g @google/gemini-cli' 'apikey' 'Free tier'
    }
    if (-not $ApiKey) {
        return New-ModelStatus 'gemini' 'Gemini' 'Google' 'key' 'yes' '' '' 'Missing GEMINI_API_KEY - run setup-keys.bat' 'apikey' 'Free tier'
    }
    $env:GEMINI_API_KEY = $ApiKey
    $env:GOOGLE_API_KEY = $ApiKey
    try {
        $outFile = Join-Path $env:TEMP 'ai-gemini-check.out'
        $errFile = Join-Path $env:TEMP 'ai-gemini-check.err'
        Remove-Item $outFile, $errFile -EA SilentlyContinue
        $null = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', "gemini -p `"Reply exactly: OK`" -o text 1>`"$outFile`" 2>`"$errFile`"" `
            -NoNewWindow -Wait -WorkingDirectory $env:USERPROFILE
        $chunks = @()
        if (Test-Path $outFile) { $chunks += Get-Content $outFile -EA SilentlyContinue }
        if (Test-Path $errFile) { $chunks += Get-Content $errFile -EA SilentlyContinue }
        $out = ($chunks -join "`n")
        if ($out -match '\bOK\b') {
            return New-ModelStatus 'gemini' 'Gemini' 'Google' 'ok' 'yes' 'Free ~250/day' '' 'API responded' 'apikey' 'Free tier'
        }
        if ($out -match 'quota|rate.?limit|429|RESOURCE_EXHAUSTED') {
            return New-ModelStatus 'gemini' 'Gemini' 'Google' 'wait' 'yes' 'Daily cap' 'Tomorrow 00:00 UTC' ($out.Trim() -split "`n" | Select-Object -Last 1) 'apikey' 'Free tier'
        }
        if ($out -match 'Auth|API_KEY|GEMINI_API_KEY') {
            return New-ModelStatus 'gemini' 'Gemini' 'Google' 'key' 'yes' '' '' 'Invalid or missing API key' 'apikey' 'Free tier'
        }
        return New-ModelStatus 'gemini' 'Gemini' 'Google' 'fail' 'yes' '' '' (($out.Trim() -split "`n" | Select-Object -Last 2) -join ' ') 'apikey' 'Free tier'
    } catch {
        return New-ModelStatus 'gemini' 'Gemini' 'Google' 'fail' 'yes' '' '' $_.Exception.Message 'apikey' 'Free tier'
    }
}

function Write-DashboardHtml {
    param([string]$ChatRoot, [object]$Payload)
    $template = Join-Path $ChatRoot 'dashboard-template.html'
    $out = Join-Path $ChatRoot 'dashboard.html'
    if (-not (Test-Path -LiteralPath $template)) { return }
    $json = ($Payload | ConvertTo-Json -Depth 6 -Compress)
    $html = [System.IO.File]::ReadAllText($template, [System.Text.UTF8Encoding]::new($false))
    $html = $html.Replace('__STATUS_JSON__', $json)
    [System.IO.File]::WriteAllText($out, $html, [System.Text.UTF8Encoding]::new($false))
}