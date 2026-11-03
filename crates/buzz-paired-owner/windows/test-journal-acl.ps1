# Ephemeral windows-latest canary. No production profile or identity is used.
$ErrorActionPreference = 'Stop'
$probe = 'buzzacl' + [guid]::NewGuid().ToString('N').Substring(0,10)
$base = Join-Path $env:PUBLIC $probe
$userCreated = $false
$registered = $false
$dbProcess = $null
try {
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $security = [Security.AccessControl.DirectorySecurity]::new()
    $security.SetOwner($sid)
    $security.SetAccessRuleProtection($true,$false)
    foreach ($entry in @(@($sid,'FullControl'),@('S-1-5-18','FullControl'),@('S-1-5-32-545','ReadAndExecute'))) {
        $principal = [Security.Principal.SecurityIdentifier]$entry[0]
        $security.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($principal,$entry[1],'ContainerInherit,ObjectInherit','None','Allow'))
    }
    [void][IO.Directory]::CreateDirectory($base,$security)
    $env:BUZZ_JOURNAL_BASE = $base
    $env:BUZZ_JOURNAL_PATH = Join-Path $base 'paired-hosts'
    & "$PSScriptRoot\protect-journal.ps1"
    $dir = $env:BUZZ_JOURNAL_PATH
    [IO.File]::WriteAllText((Join-Path $base 'control'),'public-control')
    # Keep SQLite itself running so WAL and SHM exist during the access probes.
    $python = @'
import os, sqlite3, time
from pathlib import Path
d = Path(os.environ["BUZZ_JOURNAL_PATH"])
db = sqlite3.connect(d / "owner.sqlite")
assert db.execute("pragma journal_mode=wal").fetchone()[0] == "wal"
db.execute("create table probe (value text)")
db.execute("insert into probe values ('disposable')")
db.commit()
(d.parent / "ready").touch()
for _ in range(120):
    if (d.parent / "done").exists():
        break
    time.sleep(1)
db.close()
'@
    [IO.File]::WriteAllText((Join-Path $base 'probe.py'),$python)
    $dbProcess = Start-Process -FilePath (Get-Command python.exe).Source -ArgumentList 'probe.py' -WorkingDirectory $base -PassThru
    for ($i=0; $i -lt 60 -and !(Test-Path "$base\ready"); $i++) {
        if ($dbProcess.HasExited) { throw 'SQLite probe failed' }
        Start-Sleep -Milliseconds 250
    }
    $files = @('owner.sqlite','owner.sqlite-wal','owner.sqlite-shm')
    foreach ($name in $files) {
        if (!(Test-Path -LiteralPath (Join-Path $dir $name) -PathType Leaf)) { throw 'missing SQLite file for ACL probe' }
    }
    $password = ConvertTo-SecureString ('T9!' + [guid]::NewGuid().ToString('N')) -AsPlainText -Force
    $account = New-LocalUser -Name $probe -Password $password -Description 'Disposable Buzz ACL test'
    $userCreated = $true
    $credential = [pscredential]::new("$env:COMPUTERNAME\$probe",$password)
    [IO.File]::WriteAllText((Join-Path $base 'expected-sid'),$account.SID.Value)
    $check = @'
$ErrorActionPreference = 'Stop'
if ([Security.Principal.WindowsIdentity]::GetCurrent().User.Value -ne [IO.File]::ReadAllText("$PSScriptRoot\expected-sid")) { exit 9 }
if ([IO.File]::ReadAllText("$PSScriptRoot\control") -ne 'public-control') { exit 10 }
foreach ($file in @('owner.sqlite','owner.sqlite-wal','owner.sqlite-shm')) {
    try { [IO.File]::OpenRead((Join-Path "$PSScriptRoot\paired-hosts" $file)).Dispose(); exit 11 }
    catch { if ($_.Exception.GetBaseException() -isnot [UnauthorizedAccessException]) { exit 12 } }
}
exit 0
'@
    $script = Join-Path $base 'read-probe.ps1'
    [IO.File]::WriteAllText($script,$check)
    $action = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -Argument ('-NoProfile -NonInteractive -ExecutionPolicy Bypass -File "{0}"' -f $script) -WorkingDirectory $base
    # Password logon through Task Scheduler works in a noninteractive runner.
    # Only the fixed probe path appears in the action and its process command line.
    Register-ScheduledTask -TaskName $probe -Action $action -User $credential.UserName -Password ($credential.GetNetworkCredential().Password) -RunLevel Limited | Out-Null
    $registered = $true
    $prior = (Get-ScheduledTaskInfo -TaskName $probe).LastRunTime
    Start-ScheduledTask -TaskName $probe
    $deadline = [DateTime]::UtcNow.AddSeconds(45)
    do {
        Start-Sleep -Milliseconds 250
        $info = Get-ScheduledTaskInfo -TaskName $probe
        $state = (Get-ScheduledTask -TaskName $probe).State
        $finished = $info.LastRunTime -gt $prior -and $state -eq 'Ready' -and $info.LastTaskResult -ne 267009
    } until ($finished -or [DateTime]::UtcNow -ge $deadline)
    if (!$finished -or $info.LastTaskResult -ne 0) { throw "secondary-user scheduled probe failed (state $state, result $($info.LastTaskResult))" }
    if ($dbProcess.HasExited) { throw 'SQLite closed before the secondary-account probe finished' }
    Write-Output 'Owner used SQLite; another standard user read the control and was denied the DB, WAL and SHM.'
} finally {
    if ($registered) {
        Stop-ScheduledTask -TaskName $probe -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $probe -Confirm:$false
    }
    if ($dbProcess -and !$dbProcess.HasExited) {
        [IO.File]::WriteAllText((Join-Path $base 'done'),'done')
        if (!$dbProcess.WaitForExit(5000)) { $dbProcess.Kill(); $dbProcess.WaitForExit() }
    }
    if ($userCreated) { Remove-LocalUser -Name $probe }
    if (Test-Path -LiteralPath $base) { Remove-Item -LiteralPath $base -Recurse -Force }
}
