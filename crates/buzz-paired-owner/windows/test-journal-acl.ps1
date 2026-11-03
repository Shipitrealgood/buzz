# Ephemeral windows-latest canary. No production profile or identity is used.
$ErrorActionPreference = 'Stop'
$probe = 'buzzacl' + [guid]::NewGuid().ToString('N').Substring(0,10)
$base = Join-Path $env:PUBLIC $probe
$userCreated = $false
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
    # Pass only a base64 path. No user string is interpolated into a command.
    $encodedBase = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($base))
    $check = @'
$ErrorActionPreference = 'Stop'
$base = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('ENCODED_BASE'))
if ([IO.File]::ReadAllText((Join-Path $base 'control')) -ne 'public-control') { exit 10 }
foreach ($file in @('owner.sqlite','owner.sqlite-wal','owner.sqlite-shm')) {
    try { [IO.File]::OpenRead((Join-Path "$base\paired-hosts" $file)).Dispose(); exit 11 }
    catch { if ($_.Exception.GetBaseException() -isnot [UnauthorizedAccessException]) { exit 12 } }
}
exit 0
'@
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($check.Replace('ENCODED_BASE',$encodedBase)))
    $test = Start-Process -FilePath "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList "-NoProfile -NonInteractive -EncodedCommand $encoded" -WorkingDirectory $base -Credential $credential -PassThru
    if (!$test.WaitForExit(30000)) { $test.Kill(); throw 'secondary-user probe timed out' }
    if ($test.ExitCode -ne 0) { throw "secondary-user ACL probe failed ($($test.ExitCode))" }
    Write-Output 'Owner used SQLite; another standard user read the control and was denied the DB, WAL and SHM.'
} finally {
    if ($dbProcess -and !$dbProcess.HasExited) {
        [IO.File]::WriteAllText((Join-Path $base 'done'),'done')
        if (!$dbProcess.WaitForExit(5000)) { $dbProcess.Kill(); $dbProcess.WaitForExit() }
    }
    if ($userCreated) { Remove-LocalUser -Name $probe }
    if (Test-Path -LiteralPath $base) { Remove-Item -LiteralPath $base -Recurse -Force }
}
