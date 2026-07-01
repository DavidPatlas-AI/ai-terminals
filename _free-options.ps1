# Free terminal AI options catalog (ASCII for PS1 encoding)
$script:FreeOptions = @(
    @{
        Id = 'gemini-free'; Name = 'Gemini CLI'; NameHe = 'Gemini'
        Priority = 1; Color = '#3b82f6'; Cost = 'FREE'
        Setup = '2 minutes'; KeyUrl = 'https://aistudio.google.com/apikey'
        Install = 'Already installed (npm)'
        Launch = 'AI-Gemini.bat'; KeyVar = 'GEMINI_API_KEY'
        Limit = '~250 requests/day (Flash)'
        Why = 'Fastest fix - free key, CLI already on your PC'
        Status = 'ready'
    }
    @{
        Id = 'groq'; Name = 'Groq + Crush'; NameHe = 'Groq'
        Priority = 2; Color = '#f97316'; Cost = 'FREE'
        Setup = '3 minutes'; KeyUrl = 'https://console.groq.com/keys'
        Install = 'npm install -g @charmland/crush'
        Launch = 'AI-Crush.bat'; KeyVar = 'GROQ_API_KEY'
        Limit = 'Free tier - fast Llama models'
        Why = 'Very fast responses, generous free tier'
        Status = 'optional'
    }
    @{
        Id = 'ollama'; Name = 'Ollama'; NameHe = 'Ollama'
        Priority = 3; Color = '#64748b'; Cost = '100% FREE'
        Setup = '5 minutes'; KeyUrl = 'https://ollama.com/download'
        Install = 'Download from ollama.com (no key needed)'
        Launch = 'AI-Ollama.bat'; KeyVar = ''
        Limit = 'Unlimited - runs on your PC'
        Why = 'Works offline, no API limits, no credit card'
        Status = 'optional'
    }
    @{
        Id = 'crush'; Name = 'Crush'; NameHe = 'Crush'
        Priority = 4; Color = '#ec4899'; Cost = 'FREE with key'
        Setup = '5 minutes'; KeyUrl = 'https://charm.sh/crush'
        Install = 'npm install -g @charmland/crush'
        Launch = 'AI-Crush.bat'; KeyVar = 'GEMINI_API_KEY or GROQ_API_KEY'
        Limit = 'Uses your free API keys'
        Why = 'Beautiful terminal agent - Grok/OpenCode alternative'
        Status = 'optional'
    }
    @{
        Id = 'openrouter'; Name = 'OpenRouter'; NameHe = 'OpenRouter'
        Priority = 5; Color = '#a855f7'; Cost = 'FREE models'
        Setup = '3 minutes'; KeyUrl = 'https://openrouter.ai/keys'
        Install = 'Use with Crush or LLM CLI'
        Launch = 'AI-Crush.bat'; KeyVar = 'OPENROUTER_API_KEY'
        Limit = 'Some free models (Llama, etc)'
        Why = 'Many models, some completely free'
        Status = 'optional'
    }
    @{
        Id = 'opencode'; Name = 'OpenCode'; NameHe = 'OpenCode'
        Priority = 6; Color = '#14b8a6'; Cost = 'FREE with key'
        Setup = '5 minutes'; KeyUrl = 'https://opencode.ai'
        Install = 'npm install -g opencode-ai'
        Launch = 'AI-OpenCode.bat'; KeyVar = 'GEMINI_API_KEY or GROQ_API_KEY'
        Limit = 'Multi-provider terminal agent'
        Why = 'Open source coding agent in terminal'
        Status = 'optional'
    }
    @{
        Id = 'huggingface'; Name = 'Hugging Face'; NameHe = 'HuggingFace'
        Priority = 7; Color = '#fbbf24'; Cost = 'FREE tier'
        Setup = '3 minutes'; KeyUrl = 'https://huggingface.co/settings/tokens'
        Install = 'Use with Crush (HF_TOKEN)'
        Launch = 'AI-Crush.bat'; KeyVar = 'HF_TOKEN'
        Limit = 'Free inference API limits'
        Why = 'Open models, free token'
        Status = 'optional'
    }
    @{
        Id = 'llm-cli'; Name = 'LLM CLI'; NameHe = 'LLM CLI'
        Priority = 8; Color = '#6366f1'; Cost = 'FREE'
        Setup = '2 minutes'; KeyUrl = 'https://llm.datasette.io'
        Install = 'pip install llm'
        Launch = ''; KeyVar = 'plugin keys'
        Limit = 'Plugins for Gemini, Groq, Ollama...'
        Why = 'Lightweight - connect any free API'
        Status = 'optional'
    }
    @{
        Id = 'aider'; Name = 'Aider'; NameHe = 'Aider'
        Priority = 9; Color = '#10b981'; Cost = 'FREE with Ollama'
        Setup = '10 minutes'; KeyUrl = 'https://aider.chat'
        Install = 'pip install aider-chat + Ollama'
        Launch = 'AI-Aider.bat'; KeyVar = 'OLLAMA or API keys'
        Limit = 'Best with local Ollama - free forever'
        Why = 'Coding assistant - works great with Ollama'
        Status = 'optional'
    }
)

function Get-FreeOptionDetect {
    $d = @{}
    $d.gemini_cli = [bool](Get-Command gemini -EA SilentlyContinue)
    $d.crush = [bool](Get-Command crush -EA SilentlyContinue)
    $d.opencode = [bool](Get-Command opencode -EA SilentlyContinue)
    $d.ollama = [bool](Get-Command ollama -EA SilentlyContinue)
    $d.aider = [bool](Get-Command aider -EA SilentlyContinue)
    $d.llm = [bool](Get-Command llm -EA SilentlyContinue)
    return $d
}

function Get-WorkingCount {
    param($Models)
    ($Models | Where-Object { $_.status -eq 'ok' }).Count
}