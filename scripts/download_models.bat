@echo off
setlocal
cd /d "%~dp0.."

if not exist "models" mkdir "models"

echo ==================================================
echo   Downloading AI Models...
echo ==================================================
echo.

echo [INFO] Downloading YOLOv8n-Pose (ONNX)...
set "MODEL_URL=https://huggingface.co/Xenova/yolov8n-pose-onnx/resolve/main/yolov8n-pose.onnx"
set "MODEL_FILE=models\yolov8n-pose.onnx"

curl -L -o "%MODEL_FILE%" "%MODEL_URL%"

if exist "%MODEL_FILE%" (
    echo [SUCCESS] Model downloaded to %MODEL_FILE%
) else (
    echo [ERROR] Download failed. Please download manually from:
    echo %MODEL_URL%
    echo and save it to models\yolov8n-pose.onnx
)

echo.
echo ==================================================
echo   Models Setup Complete!
echo ==================================================
pause
