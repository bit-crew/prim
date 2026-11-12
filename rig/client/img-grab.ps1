<#
.SYNOPSIS
    Push the image in the Windows clipboard to the VPS inbox.

.DESCRIPTION
    The clipboard lives on the client and the agent lives on the VPS, so an
    image has to travel as a FILE and its PATH travels as TEXT (pasting text
    over SSH works natively; OSC 52 is only needed in the reverse direction).

    This script grabs the image, uploads it to the ephemeral inbox on the
    VPS (~/.cache/prim/inbox, disposable, outside ~/work so a screenshot
    never reaches a repo or the SMB share), moves the "last" pointer to it,
    prints the remote path and copies that path into the local clipboard as
    text so the next paste puts the path in the prompt.

    It is the Windows counterpart of rig/client/img-grab and keeps the same
    contract and the same flags.

    NOTE: there is no Windows runner in CI, so this script is not linted nor
    exercised automatically, and it has not been run on a real Windows box
    yet. Keep it simple and defensive.

.EXAMPLE
    .\img-grab.ps1
    .\img-grab.ps1 --host vps --dry-run
    .\img-grab.ps1 --file C:\Users\me\capture.png
#>
[CmdletBinding()]
param(
    [Alias('host')][string]$Remote = 'vps',
    [Alias('inbox')][string]$Inbox = '~/.cache/prim/inbox',
    [Alias('file')][string]$File,
    [Alias('dry-run')][switch]$DryRun
)

$ErrorActionPreference = 'Stop'

function Write-Log([string]$Message) { Write-Host "[img-grab] $Message" }
function Fail([string]$Message) { Write-Error "[img-grab] $Message"; exit 1 }

foreach ($tool in @('ssh', 'scp')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Fail "$tool not found in PATH (needed to reach the VPS)"
    }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("img-grab-" + [System.Guid]::NewGuid().ToString('N') + '.png')

try {
    if ($File) {
        if (-not (Test-Path -LiteralPath $File)) { Fail "file not found: $File" }
        Copy-Item -LiteralPath $File -Destination $tmp
    }
    else {
        Add-Type -AssemblyName System.Windows.Forms
        Add-Type -AssemblyName System.Drawing
        $img = [System.Windows.Forms.Clipboard]::GetImage()
        if ($null -eq $img) { Fail "the clipboard does not hold an image" }
        $img.Save($tmp, [System.Drawing.Imaging.ImageFormat]::Png)
        $img.Dispose()
    }

    $bytes = [System.IO.File]::ReadAllBytes($tmp)
    if ($bytes.Length -lt 8 -or $bytes[0] -ne 0x89 -or $bytes[1] -ne 0x50 -or $bytes[2] -ne 0x4E -or $bytes[3] -ne 0x47) {
        Fail "the clipboard does not hold a PNG"
    }

    if ($File) { $base = [System.IO.Path]::GetFileNameWithoutExtension($File) } else { $base = 'clipboard' }
    $slug = ($base -replace '[^A-Za-z0-9._-]', '-')
    if ($slug.Length -gt 40) { $slug = $slug.Substring(0, 40) }
    $stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmss')
    $name = "$stamp-$slug.png"

    if ($DryRun) {
        Write-Log "dry run: would upload to ${Remote}:$Inbox/$name"
        Write-Log "dry run: would point ${Remote}:$Inbox/last at $name"
        exit 0
    }

    # The inbox lives on the VPS: a leading ~ belongs to the REMOTE home.
    $remoteHome = (& ssh $Remote 'printf %s "$HOME"')
    if ($LASTEXITCODE -ne 0 -or -not $remoteHome) { Fail "could not read the remote home from $Remote" }
    $remoteHome = $remoteHome.Trim()
    if ($Inbox -eq '~') { $Inbox = $remoteHome }
    elseif ($Inbox.StartsWith('~/')) { $Inbox = $remoteHome + $Inbox.Substring(1) }
    $remotePath = "$Inbox/$name"

    & ssh $Remote "mkdir -p '$Inbox'"
    if ($LASTEXITCODE -ne 0) { Fail "could not create the inbox on $Remote" }

    & scp $tmp "${Remote}:'$remotePath'"
    if ($LASTEXITCODE -ne 0) { Fail "upload to $Remote failed" }

    & ssh $Remote "ln -sfn '$name' '$Inbox/last'"
    if ($LASTEXITCODE -ne 0) { Fail "could not move the last pointer on $Remote" }

    Write-Log "uploaded: ${Remote}:$remotePath"
    Write-Output $remotePath

    Set-Clipboard -Value $remotePath
    Write-Log "the remote path is in your clipboard: paste it in the session"
}
finally {
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
}
