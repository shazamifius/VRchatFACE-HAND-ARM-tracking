"""
Download MediaPipe Face Mesh model and convert to ONNX format.

This script downloads the official MediaPipe Face Landmarker model and prepares it for use.
"""

import urllib.request
import os

def download_mediapipe_model():
    """Download MediaPipe Face Landmarker model (.task file - TFLite format)."""
    
    url = "https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task"
    output_path = "face_landmarker.task"
    
    print("Downloading MediaPipe Face Landmarker from Google...")
    print(f"URL: {url}")
    
    try:
        urllib.request.urlretrieve(url, output_path)
        file_size = os.path.getsize(output_path) / (1024 * 1024)  # MB
        print("Downloaded successfully!")
        print(f"Size: {file_size:.2f} MB")
        print(f"Location: {os.path.abspath(output_path)}")
        
        print("\nNOTE:")
        print("This is a .task file (TFLite format), not ONNX.")
        print("For ONNX, we'll use the stub mode for now and can:")
        print("1. Convert this to ONNX using ai.google.dev/edge/mediapipe tools")
        print("2. Or use a pre-converted ONNX model from another source")
        print("3. Or continue with STUB mode (current setup)")
        
        return True
        
    except Exception as e:
        print(f"❌ Download failed: {e}")
        return False

if __name__ == "__main__":
    download_mediapipe_model()
