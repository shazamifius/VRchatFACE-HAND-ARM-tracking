
import os
import sys
import shutil
import subprocess
import numpy as np

# Paths
SOURCE_MODELS_DIR = "blaze_tflite/models"
SOURCE_MODELS_DIR = "blaze_tflite/models"
HUB_MODELS_DIR = "../../hub/src-tauri/models"

MODELS_TO_PROCESS = [
    "face_detection_short_range",
    "face_landmark",
    "palm_detection_lite",
    "hand_landmark_lite"
]

def install_tf2onnx():
    print("Checking for tf2onnx...")
    try:
        import tf2onnx
    except ImportError:
        print("Installing tf2onnx...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "tf2onnx"])

def convert_and_copy():
    install_tf2onnx()
    
    if not os.path.exists(HUB_MODELS_DIR):
        print(f"Creating directory: {HUB_MODELS_DIR}")
        os.makedirs(HUB_MODELS_DIR)

    for model in MODELS_TO_PROCESS:
        tflite_src = os.path.join(SOURCE_MODELS_DIR, f"{model}.tflite")
        tflite_dst = os.path.join(HUB_MODELS_DIR, f"{model}.tflite")
        onnx_dst = os.path.join(HUB_MODELS_DIR, f"{model}.onnx")

        # 1. Copy TFLite
        if os.path.exists(tflite_src):
            print(f"Copying {model}.tflite to Hub...")
            shutil.copy2(tflite_src, tflite_dst)
        else:
            print(f"WARNING: Source {tflite_src} not found!")
            continue

        # 2. Convert to ONNX (if needed)
        if not os.path.exists(onnx_dst):
             print(f"Converting {model}.tflite to ONNX...")
             cmd = [
                sys.executable, "-m", "tf2onnx.convert",
                "--tflite", tflite_dst,
                "--output", onnx_dst,
                "--opset", "11"
             ]
             try:
                 subprocess.check_call(cmd)
                 print(f"Successfully created {onnx_dst}")
             except subprocess.CalledProcessError as e:
                 print(f"Failed to convert {model}: {e}")
        else:
            print(f"ONNX model {model}.onnx already exists.")

if __name__ == "__main__":
    convert_and_copy()
