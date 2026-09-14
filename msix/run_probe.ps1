# Register one probe variant as a loose-file package (needs Windows Developer Mode ON), launch it,
# then compare where its writes actually landed: the real path vs the package's private LocalCache.
# Never writes real application config files. Unregisters the package at the end.
#
# Caveat: a shell started by an MSIX-packaged app (for example the Claude desktop app) runs inside that app's
# package, so its view of AppData is merged with that app's private LocalCache. Seed files are therefore created
# by a process launched through explorer.exe (outside any package), and all checks use explicit paths.
param(
    [ValidateSet("A", "B")][string]$Variant = "A",
    # Must be NTFS and outside AppData: exFAT volumes are refused (0x80073CFD) and a virtualized AppData
    # staging copy may be invisible to the deployment service (0x80070003).
    [string]$StageRoot = "$env:SystemDrive\MsixProbeStage"
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path

$source = Join-Path $Root "out\$Variant"
$dir = Join-Path $StageRoot $Variant
New-Item -ItemType Directory -Force $dir | Out-Null
Copy-Item (Join-Path $source "*") $dir -Recurse -Force

# 1) Seed "existing" files from outside any package.
$seedDir = Join-Path $StageRoot "seed"
New-Item -ItemType Directory -Force $seedDir | Out-Null
Copy-Item (Join-Path $source "jarvis_msix_probe.exe") (Join-Path $seedDir "jarvis_msix_seed.exe") -Force
$seedDone = Join-Path $env:USERPROFILE ".JarvisMsixSeed\done.txt"
if (Test-Path $seedDone) { Remove-Item $seedDone -Force }
Start-Process "explorer.exe" -ArgumentList (Join-Path $seedDir "jarvis_msix_seed.exe")
$deadline = (Get-Date).AddSeconds(20)
while (-not (Test-Path $seedDone) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 300 }
Write-Host "seeded: $(Test-Path $seedDone)"

# 2) Register and launch the packaged probe.
Add-AppxPackage -Register (Join-Path $dir "AppxManifest.xml")
$pkg = Get-AppxPackage -Name "JarvisBench.MsixProbe$Variant"
$folder = "JarvisMsixProbe-$Variant"
$homeDir = Join-Path $env:USERPROFILE ".$folder"
$result = Join-Path $homeDir "result.json"
if (Test-Path $homeDir) { Remove-Item $homeDir -Recurse -Force }
Start-Process "explorer.exe" "shell:AppsFolder\$($pkg.PackageFamilyName)!Probe"
$deadline = (Get-Date).AddSeconds(30)
while (-not (Test-Path $result) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
Start-Sleep -Seconds 1

Write-Host "== probe says =="
if (Test-Path $result) { Get-Content $result -Raw | Write-Host } else { Write-Host "no result file" }

$cache = Join-Path $env:LOCALAPPDATA "Packages\$($pkg.PackageFamilyName)\LocalCache"
function Show([string]$name, [string]$real, [string]$private) {
    $r = if (Test-Path -LiteralPath $real) { "yes:" + ((Get-Content -LiteralPath $real -Raw -ErrorAction SilentlyContinue) -replace "\s", "") } else { "no" }
    $p = if ($private -and (Test-Path -LiteralPath $private)) { "yes:" + ((Get-Content -LiteralPath $private -Raw -ErrorAction SilentlyContinue) -replace "\s", "") } else { "no" }
    Write-Host ("{0,-40} real={1,-16} private={2}" -f $name, $r, $p)
}
Write-Host "== where writes landed =="
Show "new roaming file"            (Join-Path $env:APPDATA "$folder\roaming.txt")      (Join-Path $cache "Roaming\$folder\roaming.txt")
Show "new local file (token-like)" (Join-Path $env:LOCALAPPDATA "$folder\local.txt")   (Join-Path $cache "Local\$folder\local.txt")
Show "new home dot-dir file"       (Join-Path $homeDir "home.txt")                     ""
foreach ($pair in @(@("roaming", $env:APPDATA, "Roaming"), @("local", $env:LOCALAPPDATA, "Local"))) {
    $label, $base, $vfs = $pair
    foreach ($name in "create.json", "open.json", "rename.json") {
        Show "modify $label $name" (Join-Path $base "JarvisMsixSeed\$name") (Join-Path $cache "$vfs\JarvisMsixSeed\$name")
    }
}
$fake = Join-Path $env:LOCALAPPDATA "Packages\JarvisProbeFakePkg\LocalCache\Roaming"
Show "other package: modify existing" (Join-Path $fake "existing.json")      (Join-Path $cache "Local\Packages\JarvisProbeFakePkg\LocalCache\Roaming\existing.json")
Show "other package: new file"        (Join-Path $fake "new-$Variant.json")  (Join-Path $cache "Local\Packages\JarvisProbeFakePkg\LocalCache\Roaming\new-$Variant.json")
Show "child: home write"              (Join-Path $homeDir "child-home.txt")     ""
Show "child: sees parent private"     (Join-Path $homeDir "child-sees-parent.txt") ""
Show "child: new appdata file"        (Join-Path $env:LOCALAPPDATA "$folder-child\new.txt") (Join-Path $cache "Local\$folder-child\new.txt")
Show "in-package child: sees parent"  (Join-Path $homeDir "pkgchild-sees-parent.txt") ""
Show "in-package child: new appdata"  (Join-Path $env:LOCALAPPDATA "$folder-pkgchild\new.txt") (Join-Path $cache "Local\$folder-pkgchild\new.txt")

Remove-AppxPackage -Package $pkg.PackageFullName
Write-Host "unregistered $($pkg.PackageFullName)"
Write-Host "Test files were left under AppData, %LOCALAPPDATA%\Packages\JarvisProbeFakePkg and your profile (.JarvisMsix*). Remove them when done."
