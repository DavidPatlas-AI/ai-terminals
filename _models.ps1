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
    @{
        Id = 'notebooklm'; Name = 'NotebookLM'; NameHe = 'NotebookLM'; Provider = 'Google'; Color = '#a855f7'
        Bat = 'NotebookLM.bat'; Launch = 'launch-notebooklm.ps1'; Auth = 'login'
        AuthHint = 'notebooklm-login.bat (Google account)'; Docs = 'https://notebooklm.google.com'
        Tier = 'Free with Google'; Notes = 'Chat with your uploaded sources | install-notebooklm.bat once'
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

# Universal launch profiles.
# Keep launch behavior here so launch-ai.ps1, old launch-*.ps1 wrappers,
# dashboard shortcuts, and batch files all agree on one model catalog.
$script:AiLaunchProfiles = @(
    @{
        Id = 'grok'; Aliases = @('xai'); Kind = 'command'
        Title = 'Grok Build'; Color = 'Magenta'; WorkId = 'grok'
        Command = 'grok'; Args = @()
        PathHints = @('%USERPROFILE%\.grok\bin')
        MissingHint = 'Run: grok update'
        Tip = 'Ctrl+M = switch model | /model deepseek-v4'
        Clipboard = 'Read CLAUDE.md and continue the project.'
    }
    @{
        Id = 'claude'; Aliases = @('anthropic'); Kind = 'command'
        Title = 'Claude Code'; Color = 'Green'; WorkId = 'claude'
        Command = 'claude'; Args = @()
        MissingHint = 'npm install -g @anthropic-ai/claude-code'
        Tip = 'Paste from clipboard | ! for shell commands'
        Clipboard = 'Read the project and continue. Work in Hebrew when I write in Hebrew.'
    }
    @{
        Id = 'gemini'; Aliases = @('google'); Kind = 'command'
        Title = 'Gemini CLI'; Color = 'Blue'; WorkId = 'gemini'
        Command = 'gemini'; Args = @('--skip-trust', '-y')
        PathHints = @('%APPDATA%\npm')
        MissingHint = 'Run: npm install -g @google/gemini-cli'
        RequireAnyEnv = @('GEMINI_API_KEY')
        MissingEnvLines = @(
            'Missing Gemini API key!',
            '1. Get key: https://aistudio.google.com/apikey',
            '2. Run setup-keys.bat and paste GEMINI_API_KEY'
        )
        Tip = 'gemini -y = YOLO | -m gemini-2.5-flash'
        Clipboard = 'Continue the project from this folder. Reply in Hebrew when I write in Hebrew.'
    }
    @{
        Id = 'deepseek'; Aliases = @('ds'); Kind = 'deepseek'
        Title = 'DeepSeek via Claude Code'; Color = 'Blue'; WorkId = 'deepseek'
        Command = 'claude'
        MissingHint = 'npm install -g @anthropic-ai/claude-code'
        Tip = 'Uses DeepSeek API through Claude Code compatibility'
    }
    @{
        Id = 'codex'; Aliases = @('openai'); Kind = 'command'
        Title = 'OpenAI Codex'; Color = 'Yellow'; WorkId = 'codex'; ToolId = 'codex'
        Command = 'codex'; Args = @()
        MissingHint = 'Install from https://openai.com/codex'
        Tip = 'Tab to approve commands | sandbox on'
        Clipboard = 'Continue the project from this folder.'
        AddCodexPath = $true
    }
    @{
        Id = 'notebooklm'; Aliases = @('notebook', 'nblm'); Kind = 'delegate'
        Title = 'NotebookLM'; Color = 'Magenta'; WorkId = 'notebooklm'
        Script = 'launch-notebooklm.ps1'
        Tip = 'ask your notebook | /help for commands'
    }
    @{
        Id = 'crush'; Aliases = @('groq'); Kind = 'command'
        Title = 'Crush'; Color = 'Magenta'; WorkId = 'crush'
        Command = 'crush'; Args = @()
        MissingHint = 'Run: npm install -g @charmland/crush'
        RequireAnyEnv = @('GEMINI_API_KEY', 'GROQ_API_KEY', 'OPENROUTER_API_KEY')
        MissingEnvLines = @(
            'Need a free API key!',
            'Gemini: https://aistudio.google.com/apikey',
            'Groq:   https://console.groq.com/keys',
            'Then run: setup-keys.bat'
        )
        Tip = 'Ctrl+O switch model | works with Groq/Gemini free keys'
        Clipboard = 'Continue the project. Reply in Hebrew when I write in Hebrew.'
    }
    @{
        Id = 'ollama'; Aliases = @('local'); Kind = 'ollama'
        Title = 'Ollama'; Color = 'DarkGray'; WorkId = 'ollama'
        Command = 'ollama'; Args = @('run', 'llama3.2')
        MissingHint = 'Download: https://ollama.com/download'
        Tip = '100% free local AI - no API key needed'
        Clipboard = 'Help me with my project. Reply in Hebrew.'
    }
    @{
        Id = 'opencode'; Aliases = @('open-code'); Kind = 'command'
        Title = 'OpenCode'; Color = 'Cyan'; WorkId = 'opencode'
        Command = 'opencode'; Args = @()
        MissingHint = 'Run: npm install -g opencode-ai'
        RequireAnyEnv = @('GEMINI_API_KEY', 'GROQ_API_KEY')
        MissingEnvLines = @(
            'Need a free API key - run setup-keys.bat',
            'Gemini: https://aistudio.google.com/apikey',
            'Groq:   https://console.groq.com/keys'
        )
        Tip = 'Ctrl+O switch model | free with Gemini/Groq key'
        Clipboard = 'Continue the project from this folder.'
    }
)
