dir = CreateObject("Scripting.FileSystemObject").GetParentFolderName(WScript.ScriptFullName)
ps1 = dir & "\inject-rtl-streamdown-fix.ps1"
args = "-NoProfile -ExecutionPolicy Bypass -File """ & ps1 & """"
CreateObject("Shell.Application").ShellExecute "powershell.exe", args, "", "runas", 1
WScript.Sleep 3000
MsgBox "If you clicked YES:" & vbCrLf & "1. Close ALL Cursor" & vbCrLf & "2. Reopen Cursor" & vbCrLf & vbCrLf & "Fixes Streamdown/Glass chat Hebrew.", vbInformation, "RTL V2"