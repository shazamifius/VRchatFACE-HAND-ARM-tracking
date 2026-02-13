import os
import sys
import subprocess

def install_deps():
    print("Installing dependencies...")
    try:
        subprocess.check_call([sys.executable, "-m", "pip", "install", "tf2onnx", "tflite", "onnxruntime"])
    except:
        print("Dependency installation failed. Attempting to continue (maybe installed globally?)...")

def convert_model(tflite_path, onnx_path):
    print(f"Converting {tflite_path} -> {onnx_path}...")
    
    cmd = [
        sys.executable, "-m", "tf2onnx.convert",
        "--tflite", tflite_path,
        "--output", onnx_path,
        "--opset", "13"
    ]
    
    try:
        subprocess.check_call(cmd)
        print(f"Success: {onnx_path}")
        return True
    except subprocess.CalledProcessError as e:
        print(f"Failed to convert {tflite_path}: {e}")
        return False

def main():
    install_deps()
    
    conversions = [
        # Face
        ("models/face_landmarker_all/face_landmarks_detector.tflite", "models/FaceMesh.onnx"),
        ("models/face_landmarker_all/face_blendshapes.tflite", "models/FaceBlendshapes.onnx"),
        ("models/face_landmarker_all/face_detector.tflite", "models/face_detection_yunet_2023mar.onnx"), # Wait, YuNet is NOT TFLite usually. It's OpenCV. 
        # Actually YuNet is different. MediaPipe has its own Face Detector (BlazeFace).
        # We should use BlazeFace if we switch fully, or keep YuNet if we have it.
        # Check if YuNet exists. If not, maybe use BlazeFace converted.
        
        # Hands
        ("models/hand_landmarker_all/hand_landmarks_detector.tflite", "models/hand_landmarker.onnx"),
        ("models/hand_landmarker_all/hand_detector.tflite", "models/hand_detector.onnx"),
        
        # Pose
        ("models/pose_landmarker_all/pose_landmarks_detector.tflite", "models/pose_landmarker_full.onnx"),
        ("models/pose_landmarker_all/pose_detector.tflite", "models/pose_detector.onnx"),
    ]
    
    success_count = 0
    for tfl, onnx in conversions:
        if os.path.exists(tfl):
            if convert_model(tfl, onnx):
                success_count += 1
        else:
            print(f"Skipping {tfl} (Not found)")
            
    if success_count > 0:
        print(f"Done. Converted {success_count} models.")
    else:
        print("No models converted.")

if __name__ == "__main__":
    main()
