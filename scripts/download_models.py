import os
import urllib.request
import ssl
import sys

# Relax SSL context
ssl._create_default_https_context = ssl._create_unverified_context

MODELS = [
    {
        "url": "https://huggingface.co/Xenova/yolov8-pose-onnx/resolve/main/yolov8n-pose.onnx",
        "path": "models/yolov8n-pose.onnx",
        "min_size": 1000000, # ~1MB
        "name": "YOLOv8-Pose"
    },
    {
        "url": "https://huggingface.co/qualcomm/Facial-Landmark-Detection/resolve/81098be47283bd8adfe5311980267889ada91c6a/Facial-Landmark-Detection.onnx",
        "path": "models/Facial-Landmark-Detection.onnx",
        "min_size": 1000000,
        "name": "Qualcomm Face Landmark Detection"
    },
    {
        "url": "https://huggingface.co/qualcomm/MediaPipe-Hand-Detection/resolve/ba9417217525fda34b1c5de2292788ac03746613/MediaPipeHandDetector.onnx",
        "path": "models/MediaPipeHandDetector.onnx",
        "min_size": 1000000,
        "name": "Qualcomm Hand Detector"
    }
]

def download_file(model):
    url = model["url"]
    path = model["path"]
    min_size = model["min_size"]
    name = model["name"]
    
    print(f"Checking {name} ({path})...")
    
    # Check if exists and is valid size
    if os.path.exists(path):
        size = os.path.getsize(path)
        if size > min_size:
            print(f"  [OK] Already exists ({size} bytes)")
            return
        else:
            print(f"  [INVALID] File too small ({size} bytes). Re-downloading...")
            try:
                os.remove(path)
            except:
                pass
    
    print(f"  [DOWNLOADING] from {url}...")
    try:
        # User-Agent is sometimes required
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            with open(path, 'wb') as f:
                f.write(response.read())
        
        size = os.path.getsize(path)
        if size < min_size:
             print(f"  [ERROR] Downloaded file is too small ({size} bytes). It might be an LFS pointer.")
             # Don't raise, just warn so other downloads proceed
        else:
             print(f"  [SUCCESS] Downloaded {size} bytes")
             
    except Exception as e:
        print(f"  [ERROR] Download failed: {e}")
        print(f"  -------------------------------------------------------------")
        print(f"  MANUAL DOWNLOAD REQUIRED:")
        print(f"  Please download the file manually from your browser:")
        print(f"  Link: {url}")
        print(f"  Save it to: {os.path.abspath(path)}")
        print(f"  -------------------------------------------------------------")

def main():
    # Ensure models dir exists
    if not os.path.exists("models"):
        os.makedirs("models")
        
    print("==================================================")
    print("   Downloading AI Models (Updated List)")
    print("==================================================")
    
    for model in MODELS:
        download_file(model)
        
    print("\nDone. Please restart the application.")

if __name__ == "__main__":
    main()
