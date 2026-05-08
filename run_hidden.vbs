' run_hidden.vbs — Run a command without showing a console window
' Usage: wscript.exe run_hidden.vbs "command" "logfile"
Set args = WScript.Arguments
If args.Count < 1 Then WScript.Quit

cmd = args(0)
logfile = ""
If args.Count >= 2 Then logfile = args(1)

Set shell = CreateObject("WScript.Shell")

If logfile <> "" Then
    shell.Run "cmd /c " & cmd & " >> D:\novel\" & logfile & " 2>&1", 0, False
Else
    shell.Run "cmd /c " & cmd, 0, False
End If
