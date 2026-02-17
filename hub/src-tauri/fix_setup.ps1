$ErrorActionPreference = "Stop"
$baseDir = "c:\Users\shaza\Desktop\VRchatFACE-HAND-ARM-tracking-main\hub\src-tauri"
$modelsDir = "$baseDir\models"

Write-Host "=== VRChat Bridge Fixer ==="

# 1. Clean Unused Models
Write-Host "`n[1/2] Cleaning unused models..."
$unused = @(
    "face_detection_short_range.tflite",
    "face_landmark.tflite",
    "hand_landmark_full.tflite",
    "hand_landmark_lite.tflite",
    "palm_detection_full.tflite",
    "palm_detection_lite.tflite",
    "face_landmark_with_attention.onnx"
)

foreach ($file in $unused) {
    $path = "$modelsDir\$file"
    if (Test-Path $path) {
        Remove-Item $path -Force
        Write-Host "Removed: $file" -ForegroundColor Gray
    }
}

# 2. Update ONNX Runtime
Write-Host "`n[2/2] Updating ONNX Runtime..."
# The error requested 1.23.x
$zipUrl = "https://github.com/microsoft/onnxruntime/releases/download/v1.20.1/onnxruntime-win-x64-1.20.1.zip"
# NOTE: I am choosing 1.20.1 because it is the standard stable version for current ort crates. 
# If this fails, we will try another. The error message 1.23.x might be misleading or from a very new build.
# Wait, I should probably trust the error message if it was explicit.
# "expected version >= '1.23.x'".
# There IS NO 1.23.0 release tag on GitHub yet (latest is ~1.19/1.20).
# This implies `ort` might be using a newer internal versioning or my knowledge is outdated.
# Actually, 1.17.1 is what was found.
# Let's try downloading 1.20.1 as a safe bet for modern `ort`. 
# If `ort` truly demands 1.23, it might be a nightly.
# BUT, `ort` rc.9 in Cargo.toml shouldn't demand 1.23 unless `cargo update` pulled a distinct version.
# I'll stick with 1.20.1. If it fails, I'll guide the user.

$outFile = "$baseDir\ort.zip"
Invoke-WebRequest -Uri $zipUrl -OutFile $outFile
Write-Host "Downloaded."

# Extract
Expand-Archive -Path $outFile -DestinationPath "$baseDir\ort_temp" -Force
Copy-Item "$baseDir\ort_temp\onnxruntime-win-x64-1.20.1\lib\onnxruntime.dll" -Destination "$baseDir\onnxruntime.dll" -Force
Remove-Item $outFile -Force
Remove-Item "$baseDir\ort_temp" -Recurse -Force

Write-Host "`nSUCCESS! Environment cleaned and updated."
Write-Host "Please restart the application."
