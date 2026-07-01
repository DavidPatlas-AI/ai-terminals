Set fso = CreateObject("Scripting.FileSystemObject")
scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)
ps1 = scriptDir & "\enable-hebrew-rtl.ps1"
If Not fso.FileExists(ps1) Then
    MsgBox "enable-hebrew-rtl.ps1 not found", vbCritical
    WScript.Quit 1
End If
args = "-NoProfile -ExecutionPolicy Bypass -File """ & ps1 & """"
CreateObject("Shell.Application").ShellExecute "powershell.exe", args, "", "runas", 1
WScript.Sleep 3000
MsgBox "If you clicked Yes on UAC:" & vbCrLf & "1. Close ALL Cursor windows" & vbCrLf & "2. Reopen Cursor" & vbCrLf & vbCrLf & "Hebrew in chat should work.", vbInformation, "Hebrew RTL Fix"