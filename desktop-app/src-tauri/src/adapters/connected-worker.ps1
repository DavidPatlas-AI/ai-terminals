$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::InputEncoding = New-Object System.Text.UTF8Encoding($false)
$OutputEncoding = [Console]::OutputEncoding
$request = [Console]::In.ReadToEnd() | ConvertFrom-Json
function Emit($event) { [Console]::Out.WriteLine(($event | ConvertTo-Json -Depth 12 -Compress)); [Console]::Out.Flush() }
function Text($value) { if ($value) { Emit @{type='text_chunk';text=[string]$value} } }
$errorFile = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString()+'.box-stderr')
$promptFile = $null
$success = $false
$sawText = $false
try {
    if (-not (Test-Path -LiteralPath $request.project -PathType Container)) { throw 'Project folder not found' }
    Set-Location -LiteralPath $request.project
    foreach ($dir in @((Join-Path $env:USERPROFILE '.local\bin'),(Join-Path $env:APPDATA 'npm'),(Join-Path $env:LOCALAPPDATA 'Programs\OpenAI\Codex\bin'),(Join-Path $env:LOCALAPPDATA 'Programs\Ollama'),(Join-Path $env:USERPROFILE '.grok\bin'))) {
        if (Test-Path -LiteralPath $dir) { $env:PATH = "$dir;$env:PATH" }
    }
    switch ($request.tool) {
        'deepseek' {
            if ($request.terminalsRoot) {
                $common = Get-ChildItem -LiteralPath $request.terminalsRoot -Directory | Where-Object Name -Like '02*' | Select-Object -First 1
                if ($common) { . (Join-Path $common.FullName '_common.ps1'); Apply-AllKeys }
            }
            if (-not $env:DEEPSEEK_API_KEY) { throw 'DeepSeek: missing API key. Configure it once in AI Terminals.' }
            $model = if ($request.model) { $request.model } else { 'deepseek-v4-flash' }
            $body = @{model=$model;messages=@(@{role='user';content=$request.prompt});max_tokens=2048} | ConvertTo-Json -Depth 8
            $result = Invoke-RestMethod -Uri 'https://api.deepseek.com/chat/completions' -Method Post -Headers @{Authorization=('Bearer '+$env:DEEPSEEK_API_KEY)} -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($body)) -TimeoutSec 90
            $answer = $result.choices[0].message.content
            if (-not $answer) { throw 'DeepSeek returned no answer' }
            Text $answer
            $success=$true
        }
        'ollama' {
            $models = Invoke-RestMethod -Uri 'http://127.0.0.1:11434/api/tags' -TimeoutSec 5
            $model = $request.model
            if (-not $model) { $model = @($models.models)[0].name }
            if (-not $model) { throw 'Ollama has no installed models. No model was downloaded.' }
            $body = @{model=$model;prompt=$request.prompt;stream=$false;options=@{num_predict=2048}} | ConvertTo-Json -Depth 8
            $result = Invoke-RestMethod -Uri 'http://127.0.0.1:11434/api/generate' -Method Post -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($body)) -TimeoutSec 150
            if (-not $result.done -or -not $result.response) { throw 'Ollama returned no completed answer' }
            Text $result.response
            $success=$true
        }
        default {
            $tool = [string]$request.tool
            if ($tool -notin @('claude','codex','gemini','grok')) { throw 'Unknown tool' }
            $command = Get-Command ($tool+'.exe') -ErrorAction SilentlyContinue | Select-Object -First 1
            if (-not $command) { $command = Get-Command $tool -ErrorAction Stop | Select-Object -First 1 }
            $binary = $command.Source
            if ($tool -eq 'claude') {
                # A chat must not inherit an unrelated provider override.
                foreach ($name in @('ANTHROPIC_BASE_URL','ANTHROPIC_AUTH_TOKEN','ANTHROPIC_MODEL','ANTHROPIC_DEFAULT_OPUS_MODEL','ANTHROPIC_DEFAULT_SONNET_MODEL','ANTHROPIC_DEFAULT_HAIKU_MODEL')) { Remove-Item "Env:$name" -ErrorAction SilentlyContinue }
                $cliArgs=@('-p','--tools','','--output-format','stream-json','--verbose','--max-turns','1','--disable-slash-commands','--strict-mcp-config')
            } elseif ($tool -eq 'codex') {
                $cliArgs=@('exec','--ephemeral','--skip-git-repo-check','--sandbox','read-only','--json','-')
            } elseif ($tool -eq 'gemini') {
                $cliArgs=@('-p','Answer the request provided on stdin. Do not use tools.','--output-format','stream-json','--approval-mode','plan')
            } else {
                $promptFile=Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString()+'.box-prompt')
                [IO.File]::WriteAllText($promptFile,[string]$request.prompt,(New-Object System.Text.UTF8Encoding($false)))
                $cliArgs=@('--prompt-file',$promptFile,'--output-format','json','--max-turns','1','--no-subagents','--disable-web-search','--tools','','--permission-mode','plan')
            }
            $lines = New-Object System.Collections.Generic.List[string]
            # Prompts go through stdin or a temporary file, never executable shell text.
            function Parse-Line([string]$line) {
                if ($tool -eq 'grok') { $lines.Add($line); return }
                try { $v=$line | ConvertFrom-Json } catch { return }
                if ($tool -eq 'claude') {
                    if ($v.type -eq 'result') {
                        $success = -not $v.is_error -and $v.subtype -eq 'success'
                        if ($success) { Text $v.result; $sawText=[bool]$v.result }
                        elseif ($v.result) { Emit @{type='error';message=[string]$v.result;fatal=$true} }
                    }
                } elseif ($tool -eq 'codex') {
                    if ($v.type -eq 'item.completed' -and $v.item.type -eq 'agent_message') { Text $v.item.text; $sawText=$true }
                    if ($v.type -eq 'turn.completed') { $success=$true }
                    if ($v.type -in @('turn.failed','error')) { $success=$false; Emit @{type='error';message=[string]$v.error.message;fatal=$true} }
                } else {
                    if ($v.type -eq 'message' -and $v.role -eq 'assistant') { Text $v.content; $sawText=$true }
                    if ($v.type -eq 'result') { $success=$v.status -eq 'success' }
                }
            }
            if ($tool -in @('claude','grok')) {
                # .NET preserves empty argv values on Windows PowerShell 5.1.
                $info = New-Object Diagnostics.ProcessStartInfo
                $info.FileName=$binary
                $info.Arguments=($cliArgs | ForEach-Object {
                    $value=[string]$_
                    if ($value -and $value -notmatch '[\s"]') { $value } else {
                        $escaped=[regex]::Replace($value,'(\\*)"','$1$1\"')
                        $escaped=[regex]::Replace($escaped,'(\\+)$','$1$1')
                        '"'+$escaped+'"'
                    }
                }) -join ' '
                $info.UseShellExecute=$false
                $info.CreateNoWindow=$true
                $info.RedirectStandardInput=$true
                $info.RedirectStandardOutput=$true
                $info.RedirectStandardError=$true
                $info.StandardOutputEncoding=[Text.Encoding]::UTF8
                $proc=New-Object Diagnostics.Process
                $proc.StartInfo=$info
                [void]$proc.Start()
                $errTask=$proc.StandardError.ReadToEndAsync()
                $proc.StandardInput.WriteLine([string]$request.prompt)
                $proc.StandardInput.Close()
                while (($line=$proc.StandardOutput.ReadLine()) -ne $null) { . Parse-Line $line }
                $proc.WaitForExit()
                [IO.File]::WriteAllText($errorFile,$errTask.Result)
                $exitCode=$proc.ExitCode
                $proc.Dispose()
            } else {
                $ErrorActionPreference = 'Continue'
                $request.prompt | & $binary @cliArgs 2> $errorFile | ForEach-Object { . Parse-Line ([string]$_) }
                $exitCode=$LASTEXITCODE
                $ErrorActionPreference = 'Stop'
            }
            if ($tool -eq 'grok' -and $exitCode -eq 0) {
                $v=($lines -join "`n") | ConvertFrom-Json
                Text $v.text
                $success=[bool]$v.text
                $sawText=$success
            }
            if ($exitCode -ne 0 -or -not $success -or -not $sawText) {
                $detail = if (Test-Path -LiteralPath $errorFile) { [IO.File]::ReadAllText($errorFile) } else { '' }
                if ($detail.Length -gt 1800) { $detail=$detail.Substring($detail.Length-1800) }
                throw "$tool did not complete a chat. $detail"
            }
        }
    }
} catch {
    Emit @{type='error';message=$_.Exception.Message;fatal=$true}
    $success=$false
} finally {
    Remove-Item -LiteralPath $errorFile -ErrorAction SilentlyContinue
    if ($promptFile) { Remove-Item -LiteralPath $promptFile -ErrorAction SilentlyContinue }
}
Emit @{type='done';exit_status=$(if ($success) {'success'} else {'error'})}
if (-not $success) { exit 1 }
