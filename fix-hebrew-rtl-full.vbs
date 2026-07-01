Set fso = CreateObject("Scripting.FileSystemObject")
dir = fso.GetParentFolderName(WScript.ScriptFullName)
ps1 = dir & "\enable-hebrew-rtl.ps1"
If Not fso.FileExists(ps1) Then
    MsgBox "enable-hebrew-rtl.ps1 not found", vbCritical
    WScript.Quit 1
End If
args = "-NoProfile -ExecutionPolicy Bypass -File """ & ps1 & """"
CreateObject("Shell.Application").ShellExecute "powershell.exe", args, "", "runas", 1
WScript.Sleep 4000
MsgBox "If you clicked YES:" & vbCrLf & vbCrLf & _
       "1. Close ALL Cursor windows" & vbCrLf & _
       "2. Reopen Cursor" & vbCrLf & vbCrLf & _
       "This version fixes Agent/Grok panel too." & vbCrLf & vbCrLf & _
       "Still broken? Run paste-rtl-now.bat and paste in Console.", vbInformation, "Hebrew RTL Full Fix"