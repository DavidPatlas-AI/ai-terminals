# AI tool work + data folder catalog (ASCII only - PS1 encoding safe)

function Get-AiHubRoot {
    if (Test-Path (Join-Path $PSScriptRoot '_paths.ps1')) {
        . (Join-Path $PSScriptRoot '_paths.ps1')
        return Get-AiTerminalsRoot
    }
    return $PSScriptRoot
}

function Get-CodexLatestFolder {
    $root = Join-Path $env:USERPROFILE 'Documents\Codex'
    if (-not (Test-Path -LiteralPath $root)) { return $null }
    $candidates = @()
    $days = Get-ChildItem -LiteralPath $root -Directory -EA SilentlyContinue |
        Where-Object { $_.Name -match '^\d{4}-\d{2}-\d{2}$' } |
        Sort-Object Name -Descending |
        Select-Object -First 5
    foreach ($day in $days) {
        $candidates += Get-ChildItem -LiteralPath $day.FullName -Directory -EA SilentlyContinue
        foreach ($sub in (Get-ChildItem -LiteralPath $day.FullName -Directory -EA SilentlyContinue)) {
            if ($sub.Name -in @('work', 'outputs')) { $candidates += $sub }
            $candidates += Get-ChildItem -LiteralPath $sub.FullName -Directory -EA SilentlyContinue |
                Where-Object { $_.Name -in @('work', 'outputs') }
        }
    }
    $best = $candidates | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($best) { return $best.FullName }
    if ($days -and $days.Count -gt 0) { return $days[0].FullName }
    return $root
}

function Resolve-WorkPath {
    param([string]$Path, [string]$Fallback)
    if ($Path -and (Test-Path -LiteralPath $Path)) { return $Path }
    if ($Fallback -and (Test-Path -LiteralPath $Fallback)) { return $Fallback }
    return $null
}

function Get-ProjectsFolder {
    if ($script:WORK_DIR_PROJECTS -and (Test-Path -LiteralPath $script:WORK_DIR_PROJECTS)) {
        return $script:WORK_DIR_PROJECTS
    }
    $desktop = [Environment]::GetFolderPath('Desktop')
    $name = -join @(
        [char]0x05E4, [char]0x05E8, [char]0x05D5, [char]0x05D9,
        [char]0x05E7, [char]0x05D8, [char]0x05D9, [char]0x05DD
    )
    $fallback = Join-Path $desktop $name
    if (Test-Path -LiteralPath $fallback) { return $fallback }
    return $null
}

function Get-AiWorkFolderCatalog {
    $hub = Get-AiHubRoot
    $cfg = Join-Path $hub 'work-folders-config.ps1'
    if (Test-Path -LiteralPath $cfg) { . $cfg }

    $projects = Get-ProjectsFolder
    $codexOut = Join-Path $env:USERPROFILE 'Documents\Codex'
    $codexLatest = Get-CodexLatestFolder

    $items = @(
        @{
            Id = 'projects'; Shortcut = 'AI Projects.lnk'
            Name = 'My Projects'; Path = $projects; Kind = 'work'; Tool = 'all'
            Tip = 'Desktop projects folder'
        }
        @{
            Id = 'hub'; Shortcut = 'AI Work - Hub.lnk'
            Name = 'AI Hub folder'
            Path = $(Join-Path ([Environment]::GetFolderPath('Desktop')) 'AI-Terminals')
            Kind = 'work'; Tool = 'hub'
            Tip = 'Scripts, dashboard, launchers'
        }
        @{
            Id = 'codex-output'; Shortcut = 'AI Work - Codex.lnk'
            Name = 'Codex outputs'; Path = $(if (Test-Path -LiteralPath $codexOut) { $codexOut } else { $null })
            Kind = 'work'; Tool = 'codex'
            Tip = 'Documents\Codex - Codex builds files here (work/outputs)'
        }
        @{
            Id = 'codex-latest'; Shortcut = 'AI Work - Codex Latest.lnk'
            Name = 'Codex latest'; Path = $codexLatest; Kind = 'work'; Tool = 'codex'
            Tip = 'Most recent Codex work or outputs folder'
        }
        @{
            Id = 'codex-data'; Shortcut = 'AI Data - Codex.lnk'
            Name = 'Codex data'; Path = Join-Path $env:USERPROFILE '.codex'
            Kind = 'data'; Tool = 'codex'
            Tip = 'Sessions, config, file-history'
        }
        @{
            Id = 'codex-app'; Shortcut = 'AI Data - Codex App.lnk'
            Name = 'Codex app data'; Path = Join-Path $env:LOCALAPPDATA 'OpenAI\Codex'
            Kind = 'data'; Tool = 'codex'
            Tip = 'Generated images, app sessions'
        }
        @{
            Id = 'grok-data'; Shortcut = 'AI Data - Grok.lnk'
            Name = 'Grok sessions'; Path = Join-Path $env:USERPROFILE '.grok\sessions'
            Kind = 'data'; Tool = 'grok'
            Tip = 'Grok chat history'
        }
        @{
            Id = 'claude-data'; Shortcut = 'AI Data - Claude.lnk'
            Name = 'Claude projects'; Path = Join-Path $env:USERPROFILE '.claude\projects'
            Kind = 'data'; Tool = 'claude'
            Tip = 'Claude Code logs'
        }
        @{
            Id = 'cursor-data'; Shortcut = 'AI Data - Cursor.lnk'
            Name = 'Cursor chats'; Path = Join-Path $env:USERPROFILE '.cursor\projects'
            Kind = 'data'; Tool = 'cursor'
            Tip = 'Cursor agent transcripts'
        }
        @{
            Id = 'notebooklm-data'; Shortcut = 'AI Data - NotebookLM.lnk'
            Name = 'NotebookLM auth'; Path = Join-Path $env:USERPROFILE '.notebooklm'
            Kind = 'data'; Tool = 'notebooklm'
            Tip = 'NotebookLM login data'
        }
    )

    $result = @()
    foreach ($it in $items) {
        if (-not $it.Path -or -not (Test-Path -LiteralPath $it.Path)) { continue }
        $result += [PSCustomObject]$it
    }
    return $result
}

function Get-ToolWorkDirectory {
    param([string]$ToolId)
    $hub = Get-AiHubRoot
    $cfg = Join-Path $hub 'work-folders-config.ps1'
    if (Test-Path -LiteralPath $cfg) { . $cfg }

    $map = @{
        codex    = $script:WORK_DIR_CODEX
        grok     = $script:WORK_DIR_GROK
        claude   = $script:WORK_DIR_CLAUDE
        gemini   = $script:WORK_DIR_GEMINI
        deepseek = $script:WORK_DIR_DEEPSEEK
        crush    = $script:WORK_DIR_CRUSH
    }
    $fb = $hub
    if ($map.ContainsKey($ToolId)) {
        $p = Resolve-WorkPath $map[$ToolId] $null
        if ($p) { return $p }
        $proj = Get-ProjectsFolder
        if ($proj) { return $proj }
    }
    return $fb
}

function New-FolderShortcut {
    param([string]$Desktop, [string]$LnkName, [string]$TargetDir, [string]$Description = '')
    if (-not $TargetDir -or -not (Test-Path -LiteralPath $TargetDir)) { return $false }
    $resolved = (Get-Item -LiteralPath $TargetDir).FullName
    $Wsh = New-Object -ComObject WScript.Shell
    $lnk = Join-Path $Desktop $LnkName
    $sc = $Wsh.CreateShortcut($lnk)
    $explorer = Join-Path $env:WINDIR 'explorer.exe'
    $sc.TargetPath = $explorer
    $sc.Arguments = "`"$resolved`""
    if ($Description) { $sc.Description = $Description }
    $sc.Save()
    return $true
}