import os
import urllib.request
import sys

MODELS_DIR = os.path.join(os.path.dirname(__file__), "blaze_tflite", "models")
os.makedirs(MODELS_DIR, exist_ok=True)

MODELS = {
    "face_detection_short_range.tflite": "https://storage.googleapis.com/mediapipe-assets/face_detection_short_range.tflite",
    "face_landmark.tflite": "https://storage.googleapis.com/mediapipe-assets/face_landmark.tflite",
    "palm_detection_lite.tflite": "https://storage.googleapis.com/mediapipe-assets/palm_detection_lite.tflite",
    "hand_landmark_lite.tflite": "https://storage.googleapis.com/mediapipe-assets/hand_landmark_lite.tflite"
}

def download_file(url, filepath):
    print(f"Downloading {url} to {filepath}...")
    try:
        import ssl
        ctx = ssl.create_default_context()
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE
        
        with urllib.request.urlopen(url, context=ctx) as response, open(filepath, 'wb') as out_file:
            data = response.read()
            out_file.write(data)
            
        print("Done.")
    except Exception as e:
        print(f"Error downloading {url}: {e}")

def main():
    print("Checking models...")
    for filename, url in MODELS.items():
        filepath = os.path.join(MODELS_DIR, filename)
        
        should_download = False
        if not os.path.exists(filepath):
            print(f"[MISSING] {filename}")
            should_download = True
        else:
            size = os.path.getsize(filepath)
            if size < 100000: # Less than 100KB is suspicious for these models
                print(f"[CORRUPT] {filename} (Size: {size} bytes)")
                should_download = True
            else:
                print(f"[OK] {filename} (Size: {size} bytes)")
        
        if should_download:
            download_file(url, filepath)

    print("All models checked.")

if __name__ == "__main__":
    main()
