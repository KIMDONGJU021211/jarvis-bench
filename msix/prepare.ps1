# Build the probe and lay out two MSIX variants (A: default virtualization, B: write virtualization disabled).
# Packing with makeappx validates the manifest schema; no Developer Mode needed for this step.
# ASCII only on purpose: Windows PowerShell 5.1 misreads BOM-less UTF-8 with non-ASCII text.
param([string]$Root = "")
if (-not $Root) { $Root = Split-Path -Parent $MyInvocation.MyCommand.Path }
$ErrorActionPreference = "Continue"   # cargo writes progress to stderr; PS 5.1 treats that as fatal under Stop

& cargo build --release --manifest-path (Join-Path $Root "Cargo.toml") 2>&1 | Out-String | Write-Host
if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }

$exe = Join-Path $Root "target\release\jarvis_msix_probe.exe"
$template = [IO.File]::ReadAllText((Join-Path $Root "AppxManifest.template.xml"))
$makeappx = (Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter makeappx.exe -ErrorAction SilentlyContinue |
    Where-Object FullName -like "*\x64\*" | Sort-Object FullName -Descending | Select-Object -First 1).FullName
if (-not $makeappx) { throw "makeappx.exe not found (install the Windows SDK)" }

Add-Type -AssemblyName System.Drawing
function New-Logo([string]$path, [int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.Clear([System.Drawing.Color]::FromArgb(255, 60, 90, 160))
    $g.Dispose(); $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
}

foreach ($variant in "A", "B") {
    $dir = Join-Path $Root "out\$variant"
    New-Item -ItemType Directory -Force (Join-Path $dir "Assets") | Out-Null
    Copy-Item $exe $dir -Force
    Copy-Item $exe (Join-Path $dir "jarvis_msix_child.exe") -Force   # in-package child (like a bundled backend)
    New-Logo (Join-Path $dir "Assets\StoreLogo.png") 50
    New-Logo (Join-Path $dir "Assets\Square150x150Logo.png") 150
    New-Logo (Join-Path $dir "Assets\Square44x44Logo.png") 44
    $virt = if ($variant -eq "B") { "<desktop6:FileSystemWriteVirtualization>disabled</desktop6:FileSystemWriteVirtualization>" } else { "" }
    $cap = if ($variant -eq "B") { '<rescap:Capability Name="unvirtualizedResources" />' } else { "" }
    $xml = $template.Replace("{{VARIANT}}", $variant).Replace("{{WRITE_VIRTUALIZATION}}", $virt).Replace("{{UNVIRTUALIZED_CAP}}", $cap)
    [IO.File]::WriteAllText((Join-Path $dir "AppxManifest.xml"), $xml, (New-Object System.Text.UTF8Encoding($false)))
    $msix = Join-Path $Root "out\MsixProbe$variant.msix"
    $log = & $makeappx pack /d $dir /p $msix /o 2>&1 | Out-String
    Write-Host "[$variant] makeappx exit=$LASTEXITCODE"
    if ($LASTEXITCODE -ne 0) { Write-Host $log }
}
