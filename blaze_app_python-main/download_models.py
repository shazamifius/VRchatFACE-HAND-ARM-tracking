import os
import urllib.request
import ssl

try:
    _create_unverified_https_context = ssl._create_unverified_context
except AttributeError:
    pass
else:
    ssl._create_default_https_context = _create_unverified_https_context

MODELS = {
    "blaze_tflite/models/face_detection_short_range.tflite": "https://storage.googleapis.com/mediapipe-assets/face_detection_short_range.tflite",
    "blaze_tflite/models/face_landmark.tflite": "https://storage.googleapis.com/mediapipe-assets/face_landmark.tflite",
    "blaze_tflite/models/palm_detection_lite.tflite": "https://storage.googleapis.com/mediapipe-assets/palm_detection_lite.tflite",
    "blaze_tflite/models/hand_landmark_lite.tflite": "https://storage.googleapis.com/mediapipe-assets/hand_landmark_lite.tflite"
}

def download_models():
    print("Downloading models...")
    for path, url in MODELS.items():
        if not os.path.exists(path):
            print(f"Downloading {path}...")
            try:
                urllib.request.urlretrieve(url, path)
            except Exception as e:
                print(f"Failed to download {path}: {e}")
        else:
            print(f"Exists: {path}")

if __name__ == "__main__":
    download_models()
