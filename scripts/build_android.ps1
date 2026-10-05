<#
.SYNOPSIS
    Automated build and packaging pipeline for bit_sr Android APK and native shared libraries.
.DESCRIPTION
    Compiles libbit_sr.so for aarch64-linux-android and x86_64-linux-android,
    copies binaries into android/app/src/main/jniLibs, and runs Gradle to produce bit_sr-debug.apk.
#>

param(
    [switch]$Release = $false,
    [string[]]$Abis = @("arm64-v8a", "x86_64")
)

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Write-Host "==> bit_sr Android Build Pipeline" -ForegroundColor Cyan
Write-Host "    Workspace Root: $RootDir"

# 1. Locate Android SDK and NDK
$SdkDir = $env:ANDROID_HOME
if (-not $SdkDir) { $SdkDir = $env:ANDROID_SDK_ROOT }
if (-not $SdkDir) {
    if (Test-Path "E:\Scoop\apps\android-clt\current") {
        $SdkDir = "E:\Scoop\apps\android-clt\current"
    } elseif (Test-Path "$env:LOCALAPPDATA\Android\Sdk") {
        $SdkDir = "$env:LOCALAPPDATA\Android\Sdk"
    }
}

$NdkDir = $env:ANDROID_NDK_HOME
if (-not $NdkDir) { $NdkDir = $env:ANDROID_NDK_ROOT }
if (-not $NdkDir) {
    $PotentialNdk = Get-ChildItem "E:\Scoop\persist\android-clt\ndk", "$SdkDir\ndk", "$env:LOCALAPPDATA\Android\Sdk\ndk" -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        Select-Object -First 1
    if ($PotentialNdk) {
        $NdkDir = $PotentialNdk.FullName
    }
}

if (-not $NdkDir -or -not (Test-Path $NdkDir)) {
    Write-Error "Android NDK could not be found. Please set ANDROID_NDK_HOME or install via sdkmanager."
}

Write-Host "    Android SDK: $SdkDir" -ForegroundColor Green
Write-Host "    Android NDK: $NdkDir" -ForegroundColor Green

$LlvmBin = Join-Path $NdkDir "toolchains\llvm\prebuilt\windows-x86_64\bin"
if (-not (Test-Path $LlvmBin)) {
    # Unix / Linux layout fallback
    $LlvmBin = Join-Path $NdkDir "toolchains/llvm/prebuilt/linux-x86_64/bin"
}

# 2. Build libbit_sr.so for each requested ABI
$ProfileFlag = if ($Release) { "--release" } else { "" }
$ProfileDir = if ($Release) { "release" } else { "debug" }

foreach ($Abi in $Abis) {
    Write-Host "`n--> Compiling libbit_sr.so for ABI: $Abi..." -ForegroundColor Yellow

    if ($Abi -eq "arm64-v8a") {
        $Target = "aarch64-linux-android"
        $ClangCmd = Join-Path $LlvmBin "aarch64-linux-android28-clang.cmd"
        if (-not (Test-Path $ClangCmd)) { $ClangCmd = Join-Path $LlvmBin "aarch64-linux-android28-clang" }
        $ClangPlusCmd = Join-Path $LlvmBin "aarch64-linux-android28-clang++.cmd"
        if (-not (Test-Path $ClangPlusCmd)) { $ClangPlusCmd = Join-Path $LlvmBin "aarch64-linux-android28-clang++" }
        $ArCmd = Join-Path $LlvmBin "llvm-ar.exe"
        if (-not (Test-Path $ArCmd)) { $ArCmd = Join-Path $LlvmBin "llvm-ar" }

        $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $ClangCmd
        $env:CC_aarch64_linux_android = $ClangCmd
        $env:CXX_aarch64_linux_android = $ClangPlusCmd
        $env:AR_aarch64_linux_android = $ArCmd
    } elseif ($Abi -eq "x86_64") {
        $Target = "x86_64-linux-android"
        $ClangCmd = Join-Path $LlvmBin "x86_64-linux-android28-clang.cmd"
        if (-not (Test-Path $ClangCmd)) { $ClangCmd = Join-Path $LlvmBin "x86_64-linux-android28-clang" }
        $ClangPlusCmd = Join-Path $LlvmBin "x86_64-linux-android28-clang++.cmd"
        if (-not (Test-Path $ClangPlusCmd)) { $ClangPlusCmd = Join-Path $LlvmBin "x86_64-linux-android28-clang++" }
        $ArCmd = Join-Path $LlvmBin "llvm-ar.exe"
        if (-not (Test-Path $ArCmd)) { $ArCmd = Join-Path $LlvmBin "llvm-ar" }

        $env:CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER = $ClangCmd
        $env:CC_x86_64_linux_android = $ClangCmd
        $env:CXX_x86_64_linux_android = $ClangPlusCmd
        $env:AR_x86_64_linux_android = $ArCmd
    } else {
        Write-Warning "Unsupported ABI: $Abi; skipping."
        continue
    }

    $BuildCmd = "cargo build -p bit_sr_engine --no-default-features --lib --target $Target"
    if ($ProfileFlag) { $BuildCmd += " $ProfileFlag" }
    
    Invoke-Expression $BuildCmd
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed for $Target"
    }

    $BuiltSo = Join-Path $RootDir "target\$Target\$ProfileDir\libbit_sr.so"
    $DestDir = Join-Path $RootDir "android\app\src\main\jniLibs\$Abi"
    New-Item -ItemType Directory -Force -Path $DestDir | Out-Null
    Copy-Item $BuiltSo (Join-Path $DestDir "libbit_sr.so") -Force
    Write-Host "    Installed: $DestDir\libbit_sr.so" -ForegroundColor Green
}

# 3. Assemble APK with Gradle
Write-Host "`n--> Assembling Android APK with Gradle..." -ForegroundColor Yellow
$AndroidDir = Join-Path $RootDir "android"
Push-Location $AndroidDir
try {
    $GradleCmd = if ($IsWindows -or $env:OS -like "*Windows*") { ".\gradlew.bat" } else { "./gradlew" }
    $GradleTask = if ($Release) { "assembleRelease" } else { "assembleDebug" }
    
    & $GradleCmd $GradleTask
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Gradle build failed"
    }
} finally {
    Pop-Location
}

# 4. Final Output Verification
$ApkSubDir = if ($Release) { "release" } else { "debug" }
$ApkName = if ($Release) { "app-release-unsigned.apk" } else { "app-debug.apk" }
$SourceApk = Join-Path $RootDir "android\app\build\outputs\apk\$ApkSubDir\$ApkName"

$TargetApkDir = Join-Path $RootDir "target"
New-Item -ItemType Directory -Force -Path $TargetApkDir | Out-Null
$FinalApk = Join-Path $TargetApkDir "bit_sr-debug.apk"
Copy-Item $SourceApk $FinalApk -Force

Write-Host "`n==> Android Build Complete!" -ForegroundColor Green
Write-Host "    Output APK: $FinalApk" -ForegroundColor Cyan
Write-Host "    File Size:  $((Get-Item $FinalApk).Length / 1MB | ForEach-Object { '{0:N2} MB' -f $_ })" -ForegroundColor Cyan
