# Smoke test for the Windows installer (app/windows/installer/doki.iss).
#
#   pwsh tools/check-windows-installer.ps1 path\to\doki-setup-X.exe
#
# Installs silently for the current user, checks the files and Start menu
# shortcut, installs again over a running Doki (the update path: Setup must
# quit it first), then uninstalls and checks that the program is gone while
# Doki's data folder is untouched. Leaves the machine as it found it, apart
# from Doki's data folder if one already existed. Meant for CI runners.
param([Parameter(Mandatory = $true)][string]$Setup)

$ErrorActionPreference = 'Stop'
$app = Join-Path $env:LOCALAPPDATA 'Programs\Doki'
$exe = Join-Path $app 'doki.exe'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Doki.lnk'
$data = Join-Path $env:APPDATA 'dev.doki\doki'
$marker = Join-Path $data 'installer-check.txt'

function Fail($message) { Write-Error "installer check: $message"; exit 1 }

function Run-Setup($file, [string[]]$arguments) {
  $p = Start-Process -FilePath $file -ArgumentList $arguments -Wait -PassThru
  if ($p.ExitCode -ne 0) { Fail "$file exited with $($p.ExitCode)" }
}

$quiet = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')

# Stands in for checkpoints and settings, which uninstalling must keep.
New-Item -ItemType Directory -Force -Path $data | Out-Null
Set-Content -Path $marker -Value 'kept'

Write-Host 'Fresh install'
Run-Setup $Setup $quiet
if (-not (Test-Path $exe)) { Fail "doki.exe not installed at $exe" }
if (-not (Test-Path (Join-Path $app 'doki_ffi.dll'))) { Fail 'engine library not installed' }
foreach ($dll in 'msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll') {
  if (-not (Test-Path (Join-Path $app $dll))) { Fail "$dll not bundled" }
}
if (-not (Test-Path $shortcut)) { Fail 'no Start menu shortcut' }
$run = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name Doki -ErrorAction SilentlyContinue).Doki
if ($run -ne "`"$exe`" --hidden") { Fail "unexpected sign-in entry: $run" }

Write-Host 'Update over a running Doki'
$doki = Start-Process -FilePath $exe -ArgumentList '--hidden' -PassThru
Start-Sleep -Seconds 8
if ($doki.HasExited) { Fail "Doki did not stay running (exit $($doki.ExitCode)), so the update path was not tested" }
Run-Setup $Setup $quiet
if (-not $doki.HasExited) { Stop-Process -Id $doki.Id -Force; Fail 'Setup did not quit the running Doki' }
if (-not (Test-Path $exe)) { Fail 'doki.exe missing after update' }

Write-Host 'Uninstall'
$uninstaller = Get-ChildItem -Path $app -Filter 'unins*.exe' | Select-Object -First 1
if ($null -eq $uninstaller) { Fail 'no uninstaller' }
Run-Setup $uninstaller.FullName $quiet
# The uninstaller copies itself to a temporary folder and finishes from there.
for ($i = 0; $i -lt 40 -and (Test-Path $exe); $i++) { Start-Sleep -Milliseconds 500 }
if (Test-Path $exe) { Fail 'doki.exe still present after uninstall' }
if (Test-Path $shortcut) { Fail 'Start menu shortcut still present after uninstall' }
if (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name Doki -ErrorAction SilentlyContinue) { Fail 'sign-in entry left behind' }
if ((Get-Content -Path $marker -ErrorAction SilentlyContinue) -ne 'kept') { Fail "uninstall touched Doki's data folder" }

Remove-Item -Path $marker
Write-Host 'Installer check passed'
