. (Join-Path $PSScriptRoot '_common.ps1')
Set-WorkDirectory
Show-ChatBanner 'Ollama' 'DarkGray' '100% free local AI - no API key needed'

if (-not (Require-Command 'ollama' 'Download: https://ollama.com/download')) { return }

$models = ollama list 2>&1 | Out-String
if ($models -notmatch 'llama|qwen|gemma|mistral|phi') {
    Write-Host 'No models yet. Pulling llama3.2 (small, fast)...' -ForegroundColor Yellow
    Write-Host 'This downloads ~2GB once.' -ForegroundColor DarkGray
    ollama pull llama3.2
}

Write-Host ''
Write-Host 'Chat: ollama run llama3.2' -ForegroundColor Cyan
Write-Host 'Or use Crush/Aider with Ollama backend' -ForegroundColor DarkGray
Write-Host ''
Set-Clipboard -Value 'Help me with my project. Reply in Hebrew.'
ollama run llama3.2