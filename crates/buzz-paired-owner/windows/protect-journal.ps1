# Fixed Windows PowerShell 5.1/.NET ACL operation. Paths arrive as data.
$ErrorActionPreference = 'Stop'
$base = [IO.Path]::GetFullPath($env:BUZZ_JOURNAL_BASE).TrimEnd('\')
$dir = [IO.Path]::GetFullPath($env:BUZZ_JOURNAL_PATH).TrimEnd('\')
if (!$dir.StartsWith($base + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'unexpected journal path' }
$me = [Security.Principal.WindowsIdentity]::GetCurrent().User
$system = [Security.Principal.SecurityIdentifier]'S-1-5-18'
$trusted = @($me.Value, $system.Value, 'S-1-5-32-544')
$modify = [Security.AccessControl.FileSystemRights]'Write, Delete, DeleteSubdirectoriesAndFiles, ChangePermissions, TakeOwnership'
function Assert-Safe([IO.FileSystemInfo]$item, [bool]$private) {
    $item.Refresh()
    if (!$item.Exists -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'unexpected journal path type' }
    $acl = $item.GetAccessControl()
    $raw = [Security.AccessControl.RawSecurityDescriptor]::new($acl.GetSecurityDescriptorBinaryForm(),0)
    if ($null -eq $raw.DiscretionaryAcl -or $trusted -notcontains $raw.Owner.Value) { throw 'untrusted journal owner or DACL' }
    foreach ($ace in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {
        if (!$private -and ($ace.PropagationFlags -band [Security.AccessControl.PropagationFlags]::InheritOnly)) { continue }
        if ($ace.AccessControlType -eq 'Allow' -and $trusted -notcontains $ace.IdentityReference.Value -and
            ($private -or ($ace.FileSystemRights -band $modify))) { throw 'journal permissions are too broad' }
    }
}
function New-PrivateAcl([bool]$directory) {
    $acl = if ($directory) { [Security.AccessControl.DirectorySecurity]::new() } else { [Security.AccessControl.FileSecurity]::new() }
    $acl.SetAccessRuleProtection($true,$false)
    $acl.SetOwner($me)
    $inherit = if ($directory) { [Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit' } else { [Security.AccessControl.InheritanceFlags]::None }
    foreach ($sid in @($me,$system)) {
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl',$inherit,'None','Allow'))
    }
    return $acl
}
# Validate each directory before using it to create the next. An unexpected
# writable ancestor, junction or pre-existing accessible DB fails closed.
$current = $base
Assert-Safe ([IO.DirectoryInfo]::new($base)) $false
foreach ($component in $dir.Substring($base.Length+1).Split('\')) {
    $current = [IO.Path]::Combine($current,$component)
    $isPrivate = $current -eq $dir
    if (![IO.Directory]::Exists($current)) {
        if ($isPrivate) { [void][IO.Directory]::CreateDirectory($current,(New-PrivateAcl $true)) }
        else { [void][IO.Directory]::CreateDirectory($current) }
    }
    Assert-Safe ([IO.DirectoryInfo]::new($current)) $isPrivate
}
$root = [IO.DirectoryInfo]::new($dir)
$root.SetAccessControl((New-PrivateAcl $true))
foreach ($file in $root.EnumerateFileSystemInfos()) {
    if ($file -isnot [IO.FileInfo] -or $file.Name -notmatch '^(owner\.sqlite|paired-owner\.db)(-(journal|shm|wal))?$|^disposable-owner\.secret$') { throw 'unexpected journal file' }
    Assert-Safe $file $true
    $file.SetAccessControl((New-PrivateAcl $false))
}
