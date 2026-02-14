$ErrorActionPreference = "Stop"

$ORT_VERSION = "1.17.3" # Matches ort v2.0-rc.9 approx
$ORT_URL = "https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/onnxruntime-win-x64-${ORT_VERSION}.zip"
$DEST_DIR = "$PSScriptRoot\..\hub\src-tauri\target\debug"
$TEMP_ZIP = "$PSScriptRoot\ort_temp.zip"

Write-Host "[INFO] Downloading ONNX Runtime v$ORT_VERSION..."
Invoke-WebRequest -Uri $ORT_URL -OutFile $TEMP_ZIP

Write-Host "[INFO] Extracting onnxruntime.dll..."
Expand-Archive -Path $TEMP_ZIP -DestinationPath "$PSScriptRoot\ort_temp" -Force

$DLL_SRC = "$PSScriptRoot\ort_temp\onnxruntime-win-x64-${ORT_VERSION}\lib\onnxruntime.dll"

if (Test-Path $DLL_SRC) {
    Copy-Item -Path $DLL_SRC -Destination "$DEST_DIR\onnxruntime.dll" -Force
    Write-Host "[SUCCESS] onnxruntime.dll copied to $DEST_DIR"
} else {
    Write-Host "[ERROR] Could not find dll in extracted folder."
}

# Cleanup
Remove-Item $TEMP_ZIP -Force
Remove-Item "$PSScriptRoot\ort_temp" -Recurse -Force
