import os
import requests
import shutil

# List of mirrors for each model. The script will try them one by one.
MODELS = {
    "face_detection_short_range.onnx": [
        "https://github.com/PINTO0309/PINTO_model_zoo/raw/main/300_face_detection/face_detection_short_range.onnx",
        "https://github.com/Kazuhito00/mediapipe-python-sample/raw/main/model/face_detection_short_range.onnx",
    ],
    "face_landmark_with_attention.onnx": [
        # PINTO
        "https://github.com/PINTO0309/PINTO_model_zoo/raw/main/282_face_landmark_with_attention/face_landmark_with_attention.onnx",
        # Kazuhito00
        "https://github.com/Kazuhito00/mediapipe-python-sample/raw/main/model/face_landmark_with_attention.onnx",
        # Axon
        "https://github.com/Axon/mediapipe-assets/raw/main/face_landmark_with_attention.onnx",
    ],
    "hand_landmark_full.onnx": [
        # Keijiro (Unity)
        "https://github.com/keijiro/HandLandmarkBarracuda/raw/main/Assets/HandLandmark.onnx",
        # Wolvic
        "https://github.com/Wolvic/wolvic/raw/master/app/src/main/assets/hand_landmark_full.onnx",
        # PINTO
        "https://github.com/PINTO0309/PINTO_model_zoo/raw/main/033_Hand_Detection_and_Tracking/hand_landmark_full.onnx",
        # Geaxgx
        "https://github.com/geaxgx/depthai_hand_tracker/raw/main/models/hand_landmark_full.onnx",
    ],
    "palm_detection_full.onnx": [
        # Keijiro
        "https://github.com/keijiro/HandLandmarkBarracuda/raw/main/Assets/PalmDetection.onnx",
        # PINTO
        "https://github.com/PINTO0309/PINTO_model_zoo/raw/main/033_Hand_Detection_and_Tracking/palm_detection_full.onnx",
    ]
}

script_dir = os.path.dirname(os.path.abspath(__file__))
models_dir = os.path.join(script_dir, "..", "models")

if not os.path.exists(models_dir):
    os.makedirs(models_dir)

print(f"Downloading models to {models_dir}...")

headers = {
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
}

for name, urls in MODELS.items():
    dest = os.path.join(models_dir, name)
    print(f"\nChecking {name}...")
    
    # Check existing
    if os.path.exists(dest):
        size = os.path.getsize(dest)
        if size < 500000 and "detection" not in name: # Landmarks are usually > 1MB
             print(f"  Existing file too small ({size} bytes). Re-downloading...")
        elif size > 1000:
             print("  OK (Already exists)")
             continue

    success = False
    for url in urls:
        print(f"  Trying {url}...", end=" ")
        try:
            response = requests.get(url, headers=headers, stream=True, timeout=10)
            if response.status_code == 200:
                with open(dest, 'wb') as f:
                    shutil.copyfileobj(response.raw, f)
                print("Success!")
                success = True
                break
            else:
                print(f"Failed ({response.status_code})")
        except Exception as e:
            print(f"Error: {e}")
    
    if not success:
        print(f"  ERROR: Could not download {name} from any mirror.")

print("\nDone.")
input("Press Enter to exit...")
