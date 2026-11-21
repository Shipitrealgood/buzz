# Ephemeral windows-latest canary. No production profile or identity is used.
$ErrorActionPreference = 'Stop'
$probe = 'buzzacl' + [guid]::NewGuid().ToString('N').Substring(0,10)
$base = Join-Path $env:PUBLIC $probe
$userCreated = $false
$password = $null
$oldBase = $env:BUZZ_JOURNAL_BASE
$oldPath = $env:BUZZ_JOURNAL_PATH
$dbProcess = $null
try {
    $password = ConvertTo-SecureString ('T9!' + [guid]::NewGuid().ToString('N')) -AsPlainText -Force
    $account = New-LocalUser -Name $probe -Password $password -Description 'Disposable Buzz ACL test'
    $userCreated = $true
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $security = [Security.AccessControl.DirectorySecurity]::new()
    $security.SetOwner($sid)
    $security.SetAccessRuleProtection($true,$false)
    foreach ($entry in @(@($sid,'FullControl'),@('S-1-5-18','FullControl'),@($account.SID,'ReadAndExecute'))) {
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
    # All identity checks and opens execute synchronously on the same native
    # thread. The password never leaves process memory or enters an argument list.
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Security;
using System.Security.Principal;
using Microsoft.Win32.SafeHandles;

public static class BuzzJournalAclProbe {
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true, ExactSpelling = true)]
    private static extern bool LogonUserW(string user, string domain, IntPtr password,
        int logonType, int provider, out SafeAccessTokenHandle token);

    public static void Check(string user, SecureString password, string expectedSid,
        string control, string directory, string[] files) {
        const int LOGON32_LOGON_INTERACTIVE = 2;
        const int LOGON32_PROVIDER_DEFAULT = 0;
        SafeAccessTokenHandle token;
        IntPtr secret = Marshal.SecureStringToGlobalAllocUnicode(password);
        try {
            if (!LogonUserW(user, ".", secret, LOGON32_LOGON_INTERACTIVE, LOGON32_PROVIDER_DEFAULT, out token)) {
                int error = Marshal.GetLastWin32Error();
                throw new Win32Exception(error, "Local probe account logon failed with Windows error " + error);
            }
        } finally {
            Marshal.ZeroFreeGlobalAllocUnicode(secret);
        }

        using (token)
        using (WindowsIdentity owner = WindowsIdentity.GetCurrent())
        using (WindowsImpersonationContext context = WindowsIdentity.Impersonate(token.DangerousGetHandle())) {
            try {
                using (WindowsIdentity effective = WindowsIdentity.GetCurrent(true)) {
                    if (effective == null || effective.User.Value != expectedSid || effective.User.Equals(owner.User))
                        throw new InvalidOperationException("Unexpected effective thread SID");
                    if (new WindowsPrincipal(effective).IsInRole(WindowsBuiltInRole.Administrator))
                        throw new InvalidOperationException("Probe account is an administrator");
                }
                if (File.ReadAllText(control) != "public-control")
                    throw new InvalidOperationException("Standard account could not read the control");
                foreach (string file in files) {
                    try {
                        File.Open(Path.Combine(directory, file), FileMode.Open, FileAccess.Read,
                            FileShare.ReadWrite | FileShare.Delete).Dispose();
                    } catch (UnauthorizedAccessException denied) {
                        if (denied.HResult != unchecked((int)0x80070005)) throw;
                        continue;
                    }
                    throw new InvalidOperationException("Standard account read " + file);
                }
            } finally {
                context.Undo();
                using (WindowsIdentity restored = WindowsIdentity.GetCurrent()) {
                    if (!restored.User.Equals(owner.User))
                        throw new InvalidOperationException("Owner identity was not restored");
                }
            }
        }
    }
}
'@
    [BuzzJournalAclProbe]::Check($probe,$password,$account.SID.Value,(Join-Path $base 'control'),$dir,$files)
    if ($dbProcess.HasExited) { throw 'SQLite closed before the secondary-account probe finished' }
    Write-Output 'Owner used SQLite; another standard user read the control and was denied the DB, WAL and SHM.'
} finally {
    try {
        if ($dbProcess -and !$dbProcess.HasExited) {
            try {
                [IO.File]::WriteAllText((Join-Path $base 'done'),'done')
            } finally {
                if (!$dbProcess.WaitForExit(5000)) { $dbProcess.Kill(); $dbProcess.WaitForExit() }
            }
        }
    } finally {
        try {
            if ($userCreated) { Remove-LocalUser -Name $probe }
        } finally {
            if ($password) { $password.Dispose() }
            $env:BUZZ_JOURNAL_BASE = $oldBase
            $env:BUZZ_JOURNAL_PATH = $oldPath
            if (Test-Path -LiteralPath $base) { Remove-Item -LiteralPath $base -Recurse -Force }
        }
    }
}
