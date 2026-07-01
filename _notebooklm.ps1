# NotebookLM helpers (ASCII only - PS1 encoding safe)

function Get-NotebookLmPython {
    $candidates = @(
        "$env:LOCALAPPDATA\Microsoft\WindowsApps\PythonSoftwareFoundation.Python.3.12_qbz5n2kfra8p0\python.exe"
        "$env:LOCALAPPDATA\Programs\Python\Python312\python.exe"
        "$env:LOCALAPPDATA\Programs\Python\Python313\python.exe"
    )
    foreach ($p in $candidates) {
        if (-not (Test-Path -LiteralPath $p)) { continue }
        try {
            $out = & $p -c "import notebooklm; print('ok')" 2>&1 | Out-String
            if ($out -match 'ok') { return $p }
        } catch {}
    }
    $py = Get-Command python -ErrorAction SilentlyContinue
    if ($py) {
        try {
            $out = & $py.Source -c "import notebooklm; print('ok')" 2>&1 | Out-String
            if ($out -match 'ok') { return $py.Source }
        } catch {}
    }
    return $null
}

function Invoke-NotebookLm {
    param(
        [Parameter(Mandatory)][string[]]$Args,
        [switch]$NoNewWindow
    )
    $py = Get-NotebookLmPython
    if (-not $py) { return @{ ok = $false; code = 127; out = 'notebooklm-py not installed' } }
    $all = @('-m', 'notebooklm') + $Args
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $py
    $psi.Arguments = ($all | ForEach-Object {
        if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
    }) -join ' '
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    if ($env:NOTEBOOKLM_NOTEBOOK) { $psi.Environment['NOTEBOOKLM_NOTEBOOK'] = $env:NOTEBOOKLM_NOTEBOOK }
    $p = [System.Diagnostics.Process]::Start($psi)
    $stdout = $p.StandardOutput.ReadToEnd()
    $stderr = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    $text = ($stdout + $stderr).Trim()
    return @{ ok = ($p.ExitCode -eq 0); code = $p.ExitCode; out = $text }
}

function Test-NotebookLmInstalled {
    return [bool](Get-NotebookLmPython)
}

function Test-NotebookLmAuth {
    $storage = Join-Path $env:USERPROFILE '.notebooklm\profiles\default\storage_state.json'
    if (-not (Test-Path -LiteralPath $storage)) {
        return @{ ok = $false; detail = 'Not logged in - run notebooklm-login.bat once' }
    }
    $r = Invoke-NotebookLm -Args @('auth', 'check', '--quiet')
    if ($r.ok) { return @{ ok = $true; detail = 'Google auth OK' } }
    if ($r.out -match 'SID cookie.*fail|Cookies present.*fail') {
        return @{ ok = $false; detail = 'Auth expired - run notebooklm-login.bat' }
    }
    return @{ ok = $false; detail = ($r.out -split "`n" | Select-Object -Last 3) -join ' ' }
}

function Initialize-NotebookLmContext {
    param([string]$NotebookId)
    if (-not $NotebookId) { return $false }
    $env:NOTEBOOKLM_NOTEBOOK = $NotebookId
    $r = Invoke-NotebookLm -Args @('use', $NotebookId, '--force')
    return $r.ok
}