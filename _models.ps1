# AI terminal models catalog (ASCII only - PS1 encoding safe)
$script:AiModels = @(
    @{
        Id = 'grok'; Name = 'Grok'; NameHe = 'Grok'; Provider = 'xAI'; Color = '#8b5cf6'
        Bat = 'Grok.bat'; Launch = 'launch-grok.ps1'; Auth = 'login'
        AuthHint = 'grok login (OIDC)'; Docs = 'https://x.ai'
        Tier = 'xAI plan'; Notes = 'Ctrl+M switch model | /model deepseek-v4'
    }
    @{
        Id = 'claude'; Name = 'Claude'; NameHe = 'Claude'; Provider = 'Anthropic'; Color = '#22c55e'
        Bat = 'Claude.bat'; Launch = 'launch-claude.ps1'; Auth = 'login'
        AuthHint = 'Claude Code account'; Docs = 'https://claude.ai'
        Tier = 'Claude plan'; Notes = 'Paste from clipboard | ! for shell'
    }
    @{
        Id = 'gemini'; Name = 'Gemini'; NameHe = 'Gemini'; Provider = 'Google'; Color = '#3b82f6'
        Bat = 'Gemini.bat'; Launch = 'launch-gemini.ps1'; Auth = 'apikey'
        AuthHint = 'GEMINI_API_KEY in ai-secrets.ps1'; Docs = 'https://aistudio.google.com/apikey'
        Tier = 'Free ~250 req/day'; Notes = 'gemini -y for auto-approve'
    }
    @{
        Id = 'deepseek'; Name = 'DeepSeek'; NameHe = 'DeepSeek'; Provider = 'DeepSeek'; Color = '#06b6d4'
        Bat = 'DeepSeek.bat'; Launch = 'launch-deepseek.ps1'; Auth = 'apikey'
        AuthHint = 'DEEPSEEK_API_KEY in ai-secrets.ps1'; Docs = 'https://platform.deepseek.com'
        Tier = 'Pay per use'; Notes = 'Via Claude Code + DeepSeek API'
    }
    @{
        Id = 'codex'; Name = 'Codex'; NameHe = 'Codex'; Provider = 'OpenAI'; Color = '#eab308'
        Bat = 'Codex.bat'; Launch = 'launch-codex.ps1'; Auth = 'login'
        AuthHint = 'ChatGPT / Codex account'; Docs = 'https://chatgpt.com/codex'
        Tier = 'ChatGPT plan'; Notes = 'Tab to approve commands'
    }
)

$script:ExtraModels = @(
    @{
        Id = 'cursor'; Name = 'Cursor'; NameHe = 'Cursor'; Provider = 'Cursor'; Color = '#f97316'
        Auth = 'app'; AuthHint = 'Cursor IDE (like now)'; Docs = 'https://cursor.com'
        Tier = 'Cursor plan'; Notes = 'Works via IDE - not a separate terminal'
        Status = 'info'
    }
    @{
        Id = 'ollama'; Name = 'Ollama'; NameHe = 'Ollama'; Provider = 'Local'; Color = '#64748b'
        Auth = 'local'; AuthHint = 'Install from ollama.com - free local models'
        Docs = 'https://ollama.com'; Tier = 'Free - runs on PC'
        Notes = 'Not installed yet'; Status = 'optional'
    }
)