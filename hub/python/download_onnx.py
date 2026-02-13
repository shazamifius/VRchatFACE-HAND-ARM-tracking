import os
import urllib.request
import sys

# URLs for ONNX models (Source: cedro3 & others)
MODELS = {
    # Already downloaded: "face_detection_short_range.onnx": "...",
    "face_landmark.onnx": "https://raw.githubusercontent.com/cedro3/mediapipe_onnx/main/data/face_landmark.onnx",
    "palm_detection.onnx": "https://raw.githubusercontent.com/cedro3/mediapipe_onnx/main/data/palm_detection.onnx",
    "hand_landmark.onnx": "https://raw.githubusercontent.com/cedro3/mediapipe_onnx/main/data/hand_landmark.onnx"
}

# Destination: hub/src-tauri/models
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
# ../src-tauri/models
DEST_DIR = os.path.join(SCRIPT_DIR, "..", "src-tauri", "models")

def download_onnx():
    if not os.path.exists(DEST_DIR):
        print(f"Creating dir: {DEST_DIR}")
        os.makedirs(DEST_DIR, exist_ok=True)
        
    print(f"Downloading models to {DEST_DIR}...")
    
    for name, url in MODELS.items():
        dest_path = os.path.join(DEST_DIR, name)
        if os.path.exists(dest_path):
            print(f"[SKIP] {name}")
            continue
            
        print(f"[DOWNLOADING] {name} from {url}...")
        try:
            # Fake User-Agent to avoid 403 Forbidden
            req = urllib.request.Request(
                url, 
                data=None, 
                headers={
                    'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
                }
            )
            with urllib.request.urlopen(req) as response, open(dest_path, 'wb') as out_file:
                 out_file.write(response.read())
            print(f"[OK] {name}")
        except Exception as e:
            print(f"[ERROR] Failed {name}: {e}")

if __name__ == "__main__":
    download_onnx()
