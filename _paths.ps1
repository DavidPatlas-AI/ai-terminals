# Single source of truth for project folder paths (ASCII only)
function Get-AiTerminalsRoot {
    $desktop = [Environment]::GetFolderPath('Desktop')
    $root = Get-ChildItem $desktop -Directory -EA SilentlyContinue | Where-Object {
        $_.Name -ne 'AI-Terminals' -and
        (Test-Path (Join-Path $_.FullName 'hub.ps1')) -and
        (Test-Path (Join-Path $_.FullName '.git'))
    } | Select-Object -First 1 -ExpandProperty FullName
    if (-not $root) {
        $heb = -join @(
            [char]0x05DE, [char]0x05D5, [char]0x05D3, [char]0x05DC, [char]0x05D9, [char]0x05DD,
            ' ', [char]0x05D8, [char]0x05E8, [char]0x05DE, [char]0x05E0, [char]0x05D9, [char]0x05DC, [char]0x05D9, [char]0x05DD
        )
        $nested = Join-Path (Join-Path $desktop $heb) 'AI-Terminals'
        if ((Test-Path (Join-Path $nested 'hub.ps1')) -and (Test-Path (Join-Path $nested '.git'))) {
            return $nested
        }
    }
    if ($root) { return $root }
    if ($PSScriptRoot) { return $PSScriptRoot }
    return (Split-Path -Parent $MyInvocation.MyCommand.Path)
}