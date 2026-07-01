function Format-TokenCount([long]$n) {
    if ($n -ge 1000000) { return ('{0:N1}M' -f ($n / 1000000)) }
    if ($n -ge 1000) { return ('{0:N1}K' -f ($n / 1000)) }
    return "$n"
}

function Get-SessionStatus {
    param(
        [bool]$IsActive,
        [int]$ContextPct,
        [int]$ErrorCount,
        [datetime]$LastActive
    )
    if ($IsActive) {
        if ($ContextPct -ge 85) { return 'critical' }
        if ($ErrorCount -gt 0) { return 'warning' }
        return 'active'
    }
    if ($ErrorCount -gt 0) { return 'error' }
    if ($LastActive -gt (Get-Date).AddHours(-2)) { return 'recent' }
    return 'closed'
}

function Get-GrokConversations {
    $grokHome = Join-Path $env:USERPROFILE '.grok'
    $sessionsRoot = Join-Path $grokHome 'sessions'
    if (-not (Test-Path $sessionsRoot)) { return @() }

    $activeMap = @{}
    $activeFile = Join-Path $grokHome 'active_sessions.json'
    if (Test-Path $activeFile) {
        foreach ($a in (Get-Content $activeFile -Raw -Encoding UTF8 | ConvertFrom-Json)) {
            $activeMap[$a.session_id] = $a
        }
    }

    $list = @()
    foreach ($summaryPath in Get-ChildItem -LiteralPath $sessionsRoot -Recurse -Filter 'summary.json' -File -EA SilentlyContinue) {
        $dir = $summaryPath.Directory
        if (-not $dir) { continue }

        try {
            $summary = Get-Content -LiteralPath $summaryPath.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
        } catch { continue }

        $sessionId = $summary.info.id
        if (-not $sessionId) {
            $sessionId = $dir.Name
        }

        $signals = $null
        $signalsPath = Join-Path $dir.FullName 'signals.json'
        if (Test-Path -LiteralPath $signalsPath) {
            try { $signals = Get-Content -LiteralPath $signalsPath -Raw -Encoding UTF8 | ConvertFrom-Json } catch {}
        }

        $lastActive = $null
        if ($summary.last_active_at) {
            try { $lastActive = [datetime]::Parse($summary.last_active_at) } catch {}
        }
        if (-not $lastActive -and $summary.updated_at) {
            try { $lastActive = [datetime]::Parse($summary.updated_at) } catch {}
        }

        $isActive = $activeMap.ContainsKey($sessionId)
        $tokensUsed = if ($signals) { [long]$signals.contextTokensUsed } else { 0 }
        $tokensMax = if ($signals) { [long]$signals.contextWindowTokens } else { 200000 }
        $tokensPct = if ($signals) { [int]$signals.contextWindowUsage } else { 0 }
        $errors = if ($signals) { [int]$signals.errorCount } else { 0 }
        $turns = if ($signals) { [int]$signals.turnCount } else { 0 }
        $tools = if ($signals -and $signals.toolCallCount) { [int]$signals.toolCallCount } else { 0 }

        $title = $summary.generated_title
        if (-not $title) { $title = $summary.session_summary }
        if (-not $title) { $title = "Grok $sessionId".Substring(0, [Math]::Min(40, ("Grok $sessionId").Length)) }

        $cwd = $summary.info.cwd
        if (-not $cwd) { $cwd = $dir.Name -replace '%3A', ':' -replace '%5C', '\' -replace '\\[^\\]+$', '' }

        $list += [PSCustomObject]@{
            id = $sessionId
            provider = 'Grok'
            title = $title
            model = $summary.current_model_id
            status = (Get-SessionStatus -IsActive $isActive -ContextPct $tokensPct -ErrorCount $errors -LastActive $lastActive)
            tokensUsed = $tokensUsed
            tokensMax = $tokensMax
            tokensPct = $tokensPct
            tokensUsedFmt = (Format-TokenCount $tokensUsed)
            tokensMaxFmt = if ($tokensMax) { (Format-TokenCount $tokensMax) } else { 'n/a' }
            messages = [int]$summary.num_chat_messages
            turns = $turns
            tools = $tools
            errors = $errors
            cwd = $cwd
            lastActive = if ($lastActive) { $lastActive.ToString('yyyy-MM-dd HH:mm') } else { '' }
            lastActiveSort = if ($lastActive) { $lastActive } else { [datetime]::MinValue }
            agent = $summary.agent_name
            isActive = $isActive
        }
    }
    return $list
}

function Get-ClaudeConversations {
    $root = Join-Path $env:USERPROFILE '.claude\projects'
    if (-not (Test-Path $root)) { return @() }

    $list = @()
    foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -Filter '*.jsonl' -File -EA SilentlyContinue |
        Where-Object { $_.FullName -notmatch '\\subagents\\' -and $_.Directory.Name -notmatch '^agent-' } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 15) {

        $sessionId = $file.BaseName
        $inputTok = 0L; $outputTok = 0L; $lastTs = $file.LastWriteTime
        $title = ''; $model = ''; $status = 'closed'; $cwd = ''; $rateLimited = $false

        try {
            $lines = Get-Content -LiteralPath $file.FullName -Encoding UTF8 -Tail 80 -EA SilentlyContinue
            foreach ($line in $lines) {
                if (-not $line.Trim()) { continue }
                try { $obj = $line | ConvertFrom-Json } catch { continue }
                if ($obj.timestamp) {
                    try {
                        $ts = [datetime]::Parse($obj.timestamp)
                        if ($ts -gt $lastTs) { $lastTs = $ts }
                    } catch {}
                }
                if ($obj.cwd -and -not $cwd) { $cwd = $obj.cwd }
                if ($obj.error -eq 'rate_limit' -or $obj.isApiErrorMessage) { $rateLimited = $true; $status = 'error' }
                if ($obj.type -eq 'user' -and $obj.message -and -not $title) {
                    $t = $obj.message
                    if ($t -is [string]) { $title = ($t -replace '\s+', ' ').Trim().Substring(0, [Math]::Min(60, ($t -replace '\s+', ' ').Trim().Length)) }
                }
                if ($obj.message -and $obj.message.usage) {
                    $u = $obj.message.usage
                    $inputTok += [long]($u.input_tokens + $u.cache_creation_input_tokens + $u.cache_read_input_tokens)
                    $outputTok += [long]$u.output_tokens
                    if ($obj.message.model) { $model = $obj.message.model }
                }
            }
        } catch {}

        if (-not $title) { $title = "Claude $sessionId".Substring(0, 36) }
        if ($file.LastWriteTime -gt (Get-Date).AddHours(-2) -and -not $rateLimited) { $status = 'recent' }
        if ($file.LastWriteTime -gt (Get-Date).AddMinutes(-20) -and -not $rateLimited) { $status = 'active' }

        $total = $inputTok + $outputTok
        $list += [PSCustomObject]@{
            id = $sessionId
            provider = 'Claude'
            title = $title
            model = if ($model) { $model } else { 'claude' }
            status = $status
            tokensUsed = $total
            tokensMax = 200000
            tokensPct = if ($total -gt 0) { [int][Math]::Min(99, ($total / 200000) * 100) } else { 0 }
            tokensUsedFmt = (Format-TokenCount $total)
            tokensMaxFmt = '200K'
            messages = 0
            turns = 0
            tools = 0
            errors = if ($rateLimited) { 1 } else { 0 }
            cwd = $cwd
            lastActive = $lastTs.ToString('yyyy-MM-dd HH:mm')
            lastActiveSort = $lastTs
            agent = 'claude-code'
            isActive = ($status -eq 'active')
        }
    }
    return $list
}

function Get-CursorConversations {
    $root = Join-Path $env:USERPROFILE '.cursor\projects'
    if (-not (Test-Path $root)) { return @() }

    $list = @()
    foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -Filter '*.jsonl' -File -EA SilentlyContinue |
        Where-Object { $_.FullName -match 'agent-transcripts' } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 10) {

        $sessionId = $file.BaseName
        $lastTs = $file.LastWriteTime
        $title = ''; $msgCount = 0

        try {
            foreach ($line in (Get-Content -LiteralPath $file.FullName -Encoding UTF8 -Tail 30 -EA SilentlyContinue)) {
                if (-not $line.Trim()) { continue }
                try { $obj = $line | ConvertFrom-Json } catch { continue }
                $msgCount++
                if ($obj.role -eq 'user' -and $obj.message -and -not $title) {
                    $parts = $obj.message.content
                    if ($parts -is [array]) {
                        foreach ($p in $parts) {
                            if ($p.type -eq 'text' -and $p.text -match 'user_query') {
                                if ($p.text -match '<user_query>\s*(.+?)\s*</user_query>') {
                                    $title = $Matches[1].Trim()
                                    if ($title.Length -gt 60) { $title = $title.Substring(0, 60) }
                                }
                            }
                        }
                    }
                }
            }
        } catch {}

        if (-not $title) { $title = "Cursor $sessionId".Substring(0, 36) }
        $status = if ($lastTs -gt (Get-Date).AddMinutes(-30)) { 'active' }
                  elseif ($lastTs -gt (Get-Date).AddHours(-6)) { 'recent' }
                  else { 'closed' }

        $list += [PSCustomObject]@{
            id = $sessionId
            provider = 'Cursor'
            title = $title
            model = 'cursor-agent'
            status = $status
            tokensUsed = 0
            tokensMax = 0
            tokensPct = 0
            tokensUsedFmt = 'n/a'
            tokensMaxFmt = 'n/a'
            messages = $msgCount
            turns = 0
            tools = 0
            errors = 0
            cwd = ''
            lastActive = $lastTs.ToString('yyyy-MM-dd HH:mm')
            lastActiveSort = $lastTs
            agent = 'cursor'
            isActive = ($status -eq 'active')
        }
    }
    return $list
}

function Get-ConversationDashboard {
    $grok = Get-GrokConversations
    $claude = Get-ClaudeConversations
    $cursor = Get-CursorConversations

    $grokTop = @($grok | Sort-Object isActive, lastActiveSort -Descending | Select-Object -First 18)
    $claudeTop = @($claude | Sort-Object lastActiveSort -Descending | Select-Object -First 5)
    $cursorTop = @($cursor | Sort-Object lastActiveSort -Descending | Select-Object -First 2)
    $all = @($grokTop) + @($claudeTop) + @($cursorTop) |
        Sort-Object @{ Expression = 'isActive'; Descending = $true },
                    @{ Expression = 'lastActiveSort'; Descending = $true }

    $activeGrok = @($grok | Where-Object { $_.isActive })
    $tokensActive = ($activeGrok | Measure-Object -Property tokensUsed -Sum).Sum
    if (-not $tokensActive) { $tokensActive = 0 }

    $summary = [PSCustomObject]@{
        totalConversations = $all.Count
        activeCount = @($all | Where-Object { $_.status -in @('active','critical','warning') }).Count
        grokActive = $activeGrok.Count
        grokTotal = $grok.Count
        claudeRecent = @($claude | Where-Object { $_.status -ne 'closed' }).Count
        cursorRecent = @($cursor | Where-Object { $_.status -ne 'closed' }).Count
        tokensActiveUsed = $tokensActive
        tokensActiveFmt = (Format-TokenCount $tokensActive)
        highestPct = if ($grok.Count) { ($grok | Measure-Object -Property tokensPct -Maximum).Maximum } else { 0 }
        errorCount = @($all | Where-Object { $_.errors -gt 0 -or $_.status -eq 'error' }).Count
    }

    return @{
        summary = $summary
        conversations = $all
    }
}