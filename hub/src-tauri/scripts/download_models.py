import os
import urllib.request
import shutil
import sys
import ssl

# Bypass SSL verification
ssl._create_default_https_context = ssl._create_unverified_context

# List of mirrors for each model.
MODELS = {
    "face_detection_short_range.onnx": [
        "https://github.com/PINTO0309/PINTO_model_zoo/raw/main/300_face_detection/face_detection_short_range.onnx",
    ],
    "face_landmark.onnx": [
        "https://github.com/ibaiGorordo/ONNX-MediaPipe-Face-Mesh/raw/main/data/face_mesh.onnx",
        "https://raw.githubusercontent.com/cedro3/mediapipe_onnx/main/data/face_landmark.onnx",
    ],
    # NOTE: opencv_zoo stores .onnx files via Git LFS. The plain
    # raw.githubusercontent.com URL returns a ~130-byte LFS *pointer*, NOT the
    # model — you MUST use the media.githubusercontent.com/media/ endpoint, which
    # serves the real binary. (raw mirrors are kept as fallbacks.)
    "palm_detection.onnx": [
        "https://media.githubusercontent.com/media/opencv/opencv_zoo/main/models/palm_detection_mediapipe/palm_detection_mediapipe_2023feb.onnx",
        "https://github.com/Vidya-Suri/hand-gesture-recognition/raw/main/model/palm_detection.onnx",
    ],
    "hand_landmark.onnx": [
        "https://media.githubusercontent.com/media/opencv/opencv_zoo/main/models/handpose_estimation_mediapipe/handpose_estimation_mediapipe_2023feb.onnx",
        "https://github.com/Wolvic/wolvic/raw/master/app/src/main/assets/hand_landmark_full.onnx",
    ],
    # Body pose (BlazePose). Landmark model outputs 33 keypoints; detector is for
    # Phase 1.5 robustness. Both live in opencv_zoo behind Git LFS (media URL).
    "pose_landmark.onnx": [
        "https://media.githubusercontent.com/media/opencv/opencv_zoo/main/models/pose_estimation_mediapipe/pose_estimation_mediapipe_2023mar.onnx",
    ],
    "pose_detection.onnx": [
        "https://media.githubusercontent.com/media/opencv/opencv_zoo/main/models/person_detection_mediapipe/person_detection_mediapipe_2023mar.onnx",
    ],
}

script_dir = os.path.dirname(os.path.abspath(__file__))
models_dir = os.path.join(script_dir, "..", "models")

if not os.path.exists(models_dir):
    os.makedirs(models_dir)

print(f"Downloading models to {models_dir}...")

headers = {
    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
}

def download_file(url, dest):
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            with open(dest, 'wb') as out_file:
                shutil.copyfileobj(response, out_file)
        return True
    except Exception as e:
        print(f"Error downloading {url}: {e}")
        return False

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
        sys.stdout.flush()
        if download_file(url, dest):
            print("Success!")
            success = True
            break
        else:
            print("Failed")
    
    if not success:
        print(f"  ERROR: Could not download {name} from any mirror.")

print("\nDone.")
