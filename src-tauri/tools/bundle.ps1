param(
    [string]$ProductName = "Secure Link",
    [string]$Manufacturer = "4A",
    [string]$UpgradeCode = "17603da0-6641-4187-851b-598409b3f835",
    [string]$VcRedist = "installer\\VC_redist.x64.exe",
    [string]$Wxs = "installer\\Bundle.wxs"
)

# 1) Находим MSI от Tauri

$msi = Get-ChildItem -Recurse -Filter *.msi -Path ./target/x86_64-pc-windows-msvc/release/bundle/msi | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $msi) { throw "MSI not found under .\target" }

# 2) Версию берём из имени MSI или тащим сами (здесь парсим из имени по шаблону 1.2.3)
# Если имя нестандартное — задайте вручную через аргумент.
$version = ($msi.BaseName | Select-String -Pattern '\d+\.\d+\.\d+(\.\d+)?' -AllMatches).Matches.Value | Select-Object -First 1
if (-not $version) { $version = "1.0.0.0" }

# 3) Папки для obj и out
New-Item -ItemType Directory -Force -Path .\target\bundle_obj | Out-Null
New-Item -ItemType Directory -Force -Path .\target\bundle_dist | Out-Null

# 4) Пути к WiX (если в PATH, можно просто вызвать по имени)
$candle = "candle.exe"
$light  = "light.exe"

# 5) Собираем
& $candle `
  -nologo `
  -ext WixBalExtension `
  -ext WixUtilExtension `
  -dAppName="$ProductName" `
  -dManufacturer="$Manufacturer" `
  -dBundleVersion="$version" `
  -dUpgradeCode="$UpgradeCode" `
  -dMsiPath="$($msi.FullName)" `
  -dVCRedistPath="$VcRedist" `
  -out .\target\bundle_obj\Bundle.wixobj `
  $Wxs
if ($LASTEXITCODE -ne 0) { throw "candle failed" }

& $light `
  -nologo `
  -ext WixBalExtension `
  -ext WixUtilExtension `
  -out ".\target\bundle_dist\$($ProductName)-Setup-$version-x64.exe" `
  .\target\bundle_obj\Bundle.wixobj
if ($LASTEXITCODE -ne 0) { throw "light failed" }

Write-Host "Bundle created: .\target\bundle_dist\$($ProductName)-Setup-$version-x64.exe"
