# 确保终端输出编码为 UTF-8（适配 chcp 65001 环境）
$OutputEncoding = [System.Text.UTF8Encoding]::new()
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()

<#
.SYNOPSIS
编译 Windows 发布版本并重命名输出文件
.DESCRIPTION
构建 Windows (x86_64-pc-windows-msvc) 目标，输出到 dist/ 目录。
Linux 用户请直接在 Linux 环境下自行编译。
#>

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$Version = (& cargo metadata --format-version 1 --no-deps |
    ConvertFrom-Json).packages[0].version
$DistDir = "$ProjectRoot\dist"

# 清空旧的发布目录
if (Test-Path $DistDir) { Remove-Item $DistDir -Recurse -Force }
New-Item -ItemType Directory -Path $DistDir | Out-Null

Write-Host "══════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  rar-cracker v$Version  Windows Release Build" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# ══════════════════════════════════════════════════════════════════════════
#  编译 Windows x86_64
# ══════════════════════════════════════════════════════════════════════════
$Target = "x86_64-pc-windows-msvc"
Write-Host "▶ 正在编译 $Target ..." -ForegroundColor Yellow
cargo build --release --target $Target
if ($LASTEXITCODE -ne 0) {
    Write-Host "  ✘ 编译失败" -ForegroundColor Red
    exit $LASTEXITCODE
}

$Src = "$ProjectRoot\target\$Target\release\rar-cracker-rs.exe"
$Dst = "$DistDir\rar-cracker_v$Version`_$Target.exe"
Copy-Item $Src $Dst
Write-Host "  ✔ $Dst" -ForegroundColor Green
Write-Host ""

# ══════════════════════════════════════════════════════════════════════════
#  输出汇总
# ══════════════════════════════════════════════════════════════════════════
Write-Host "══════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  发布文件已输出到: $DistDir" -ForegroundColor Cyan
Get-ChildItem $DistDir | ForEach-Object {
    Write-Host "  $($_.Name)" -ForegroundColor White
}
Write-Host "══════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "  💡 Linux 用户请直接在 Linux 环境下运行 cargo build --release" -ForegroundColor Magenta