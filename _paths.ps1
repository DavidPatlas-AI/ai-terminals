# Single source of truth for project folder paths (ASCII only)
function Get-AiTerminalsRoot {
    $desktop = [Environment]::GetFolderPath('Desktop')
    $junction = Join-Path $desktop 'AI-Terminals'
    if ((Test-Path -LiteralPath (Join-Path $junction 'hub.ps1')) -and
        (Test-Path -LiteralPath (Join-Path $junction '.git'))) {
        return $junction
    }
    $root = Get-ChildItem $desktop -Directory -EA SilentlyContinue | Where-Object {
        $_.Name -ne 'AI-Terminals' -and
        (Test-Path (Join-Path $_.FullName 'hub.ps1')) -and
        (Test-Path (Join-Path $_.FullName '.git'))
    } | Select-Object -First 1 -ExpandProperty FullName
    if ($root) { return $root }
    if ($PSScriptRoot) { return $PSScriptRoot }
    return (Split-Path -Parent $MyInvocation.MyCommand.Path)
}