' One-click Hebrew RTL fix for Cursor - requests Administrator (UAC Yes required)
Set fso = CreateObject("Scripting.FileSystemObject")
scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)
ps1 = scriptDir & "\enable-hebrew-rtl.ps1"

If Not fso.FileExists(ps1) Then
    MsgBox "לא נמצא: enable-hebrew-rtl.ps1" & vbCrLf & ps1, vbCritical, "תיקון עברית"
    WScript.Quit 1
End If

args = "-NoProfile -ExecutionPolicy Bypass -File """ & ps1 & """"
CreateObject("Shell.Application").ShellExecute "powershell.exe", args, "", "runas", 1

WScript.Sleep 3000
MsgBox "אם אישרת 'כן' בחלון האדום:" & vbCrLf & vbCrLf & _
       "1. סגור את כל חלונות Cursor" & vbCrLf & _
       "2. פתח Cursor מחדש" & vbCrLf & vbCrLf & _
       "העברית בצ'אט אמורה להיות תקינה." & vbCrLf & vbCrLf & _
       "אם לא אישרת - הרץ שוב ולחץ כן.", vbInformation, "תיקון עברית ב-Cursor"