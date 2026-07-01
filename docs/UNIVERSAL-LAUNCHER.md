# Universal Launcher

`launch-ai.ps1` is the single launcher for all terminal AI tools in this
project. Old files such as `Grok.bat`, `Codex.bat`, and `launch-grok.ps1`
still work, but they now go through the same central path.

## Quick Use

```bat
AI.bat grok
AI.bat claude
AI.bat gemini
AI.bat deepseek
AI.bat codex
AI.bat crush
AI.bat ollama
AI.bat opencode
AI.bat notebooklm
```

Or from PowerShell:

```powershell
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File .\launch-ai.ps1 -Model codex
```

## Where To Configure

- Models and launch behavior: `_models.ps1`, section `AiLaunchProfiles`
- Shared environment/key logic: `_common.ps1`
- Per-tool working folders: `work-folders-config.ps1`
- Dashboard and health checks: `check-all.ps1`

## Compatibility

The old launchers are kept intentionally:

- `launch-grok.ps1`
- `launch-claude.ps1`
- `launch-gemini.ps1`
- `launch-deepseek.ps1`
- `launch-codex.ps1`
- `launch-crush.ps1`
- `launch-ollama.ps1`
- `launch-opencode.ps1`

They are now thin wrappers around `launch-ai.ps1`.
