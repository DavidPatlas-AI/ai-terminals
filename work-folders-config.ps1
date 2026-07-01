# Default work folder when launching tools from AI Hub (edit paths here)
# Use full path. Leave empty to auto-detect projects folder.

$projects = Join-Path $env:USERPROFILE 'Desktop'
$projects = Join-Path $projects ([char]0x05E4 + [char]0x05E8 + [char]0x05D5 + [char]0x05D9 + [char]0x05E7 + [char]0x05D8 + [char]0x05D9 + [char]0x05DD)

$script:WORK_DIR_CODEX    = $projects
$script:WORK_DIR_GROK     = $projects
$script:WORK_DIR_CLAUDE   = $projects
$script:WORK_DIR_GEMINI   = $projects
$script:WORK_DIR_DEEPSEEK = $projects
$script:WORK_DIR_CRUSH    = $projects
$script:WORK_DIR_PROJECTS = $projects