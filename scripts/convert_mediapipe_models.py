#!/usr/bin/env python3
"""
MediaPipe TFLite to ONNX Converter
Converts all MediaPipe models for VRChat Face/Hand/Body Tracking
"""

import os
import sys
import subprocess
import shutil
from pathlib import Path

def print_header(text):
    print(f"\n{'='*60}")
    print(f"  {text}")
    print(f"{'='*60}\n")

def run_command(cmd, description):
    """Run a command and handle errors"""
    print(f"[INFO] {description}...")
    try:
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        print(f"[OK] {description} completed successfully")
        return True
    except subprocess.CalledProcessError as e:
        print(f"[ERROR] {description} failed:")
        print(f"  Command: {' '.join(cmd)}")
        print(f"  Error: {e.stderr}")
        return False
    except FileNotFoundError:
        print(f"[ERROR] Command not found: {cmd[0]}")
        print(f"  Make sure required tools are installed")
        return False

def install_dependencies():
    """Install required Python packages"""
    print_header("Installing Dependencies")
    
    packages = [
        "tf2onnx",
        "onnx",
        "onnxruntime",
        "tensorflow"
    ]
    
    for pkg in packages:
        run_command(
            [sys.executable, "-m", "pip", "install", pkg],
            f"Installing {pkg}"
        )

def convert_tflite_to_onnx(tflite_path, onnx_path, opset=13):
    """Convert a single TFLite model to ONNX"""
    if not os.path.exists(tflite_path):
        print(f"[SKIP] {tflite_path} not found")
        return False
    
    print(f"[INFO] Converting {Path(tflite_path).name}...")
    print(f"       Input:  {tflite_path}")
    print(f"       Output: {onnx_path}")
    
    python_exe = r"C:/Users/shaza/AppData/Local/Programs/Python/Python313/python.exe"
    cmd = [
        python_exe, "-m", "tf2onnx.convert",
        "--tflite", tflite_path,
        "--output", onnx_path,
        "--opset", str(opset)
    ]
    
    try:
        subprocess.run(cmd, check=True, capture_output=True, text=True)
        
        # Verify output exists
        if os.path.exists(onnx_path):
            size_mb = os.path.getsize(onnx_path) / (1024 * 1024)
            print(f"[OK] Created {onnx_path} ({size_mb:.2f} MB)")
            return True
        else:
            print(f"[ERROR] Output file not created")
            return False
            
    except subprocess.CalledProcessError as e:
        print(f"[ERROR] Conversion failed: {e.stderr}")
        return False

def optimize_onnx_model(onnx_path):
    """Optimize ONNX model for faster inference"""
    print(f"[INFO] Optimizing {Path(onnx_path).name}...")
    
    try:
        import onnx
        from onnx import optimizer
        
        # Load model
        model = onnx.load(onnx_path)
        
        # Apply optimizations
        passes = [
            'eliminate_identity',
            'eliminate_nop_transpose',
            'eliminate_nop_pad',
            'fuse_bn_into_conv',
            'fuse_consecutive_transposes',
            'fuse_transpose_into_gemm',
        ]
        
        optimized_model = optimizer.optimize(model, passes)
        
        # Save optimized model
        onnx.save(optimized_model, onnx_path)
        print(f"[OK] Model optimized")
        return True
        
    except Exception as e:
        print(f"[WARNING] Optimization failed (model still usable): {e}")
        return False

def inspect_onnx_model(onnx_path):
    """Print model input/output information"""
    try:
        import onnxruntime as ort
        
        sess = ort.InferenceSession(onnx_path, providers=['CPUExecutionProvider'])
        
        print(f"\n  Model: {Path(onnx_path).name}")
        print(f"  Inputs:")
        for inp in sess.get_inputs():
            print(f"    - {inp.name}: {inp.shape} ({inp.type})")
        print(f"  Outputs:")
        for out in sess.get_outputs():
            print(f"    - {out.name}: {out.shape} ({out.type})")
            
        return True
    except Exception as e:
        print(f"[WARNING] Could not inspect model: {e}")
        return False

def main():
    print_header("MediaPipe TFLite -> ONNX Converter")
    
    # Define base paths
    models_dir = Path("models")
    
    # Model conversion list
    conversions = [
        # Face Tracking
        {
            "name": "Face Detector (BlazeFace)",
            "input": models_dir / "face_landmarker_all" / "face_detector.tflite",
            "output": models_dir / "face_detector_blazeface.onnx"
        },
        {
            "name": "Face Landmarks Detector",
            "input": models_dir / "face_landmarker_all" / "face_landmarks_detector.tflite",
            "output": models_dir / "face_landmarks.onnx"
        },
        {
            "name": "Face Blendshapes",
            "input": models_dir / "face_landmarker_all" / "face_blendshapes.tflite",
            "output": models_dir / "face_blendshapes.onnx"
        },
        
        # Hand Tracking
        {
            "name": "Hand Detector (PalmDet)",
            "input": models_dir / "hand_landmarker_all" / "hand_detector.tflite",
            "output": models_dir / "hand_detector_palmdet.onnx"
        },
        {
            "name": "Hand Landmarks Detector",
            "input": models_dir / "hand_landmarker_all" / "hand_landmarks_detector.tflite",
            "output": models_dir / "hand_landmarks.onnx"
        },
        
        # Pose Tracking
        {
            "name": "Pose Detector (BlazePose)",
            "input": models_dir / "pose_landmarker_all" / "pose_detector.tflite",
            "output": models_dir / "pose_detector_blazepose.onnx"
        },
        {
            "name": "Pose Landmarks Detector (Full)",
            "input": models_dir / "pose_landmarker_all" / "pose_landmarks_detector.tflite",
            "output": models_dir / "pose_landmarks_mediapipe.onnx"
        },
    ]
    
    # Install dependencies
    # install_dependencies()
    
    # Download YuNet
    print_header("Downloading YuNet")
    yunet_url = "https://github.com/opencv/opencv_zoo/raw/main/models/face_detection_yunet/face_detection_yunet_2023mar.onnx"
    yunet_path = models_dir / "face_detection_yunet_2023mar.onnx"
    
    import urllib.request
    try:
        print(f"[INFO] Downloading YuNet from {yunet_url}...")
        urllib.request.urlretrieve(yunet_url, yunet_path)
        print(f"[OK] Downloaded {yunet_path}")
    except Exception as e:
        print(f"[ERROR] Failed to download YuNet: {e}")
        # Not fatal, user might have it
    
    # Convert models
    print_header("Converting Models")
    
    success_count = 0
    failed_models = []
    
    for conv in conversions:
        print(f"\n{'-'*60}")
        print(f"Converting: {conv['name']}")
        print(f"{'-'*60}")
        
        if convert_tflite_to_onnx(str(conv['input']), str(conv['output'])):
            # Optimize the model
            optimize_onnx_model(str(conv['output']))
            
            # Inspect the model
            inspect_onnx_model(str(conv['output']))
            
            success_count += 1
        else:
            failed_models.append(conv['name'])
    
    # Summary
    print_header("Conversion Summary")
    print(f"[OK] Successfully converted: {success_count}/{len(conversions)}")
    if os.path.exists(yunet_path):
        print(f"[OK] YuNet status: Present")
    else:
        print(f"[FAIL] YuNet status: Missing")
    
    if failed_models:
        print(f"\n[FAIL] Failed conversions:")
        for name in failed_models:
            print(f"   - {name}")
        return 1
    else:
        print(f"\n[DONE] All models converted successfully!")
        print(f"\nNext steps:")
        print(f"  1. Rebuild the C++ project (cmake --build build --config Release)")
        print(f"  2. Test the new models")
        print(f"  3. Check GPU acceleration logs")
        return 0
        print(f"\nNext steps:")
        print(f"  1. Rebuild the C++ project (cmake --build build --config Release)")
        print(f"  2. Test the new models")
        print(f"  3. Check GPU acceleration logs")
        return 0

if __name__ == "__main__":
    try:
        exit_code = main()
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print(f"\n\n[INFO] Conversion cancelled by user")
        sys.exit(130)
    except Exception as e:
        print(f"\n\n[FATAL ERROR] {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
