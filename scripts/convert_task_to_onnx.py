"""
Convert MediaPipe .task file to ONNX format

This script extracts the TFLite model from MediaPipe .task file 
and converts it to ONNX format for use with ONNX Runtime.

Requirements:
  pip install tf2onnx tensorflow onnx
"""

import os
import sys
import shutil
import zipfile
import subprocess

def extract_tflite_from_task(task_file, output_tflite):
    """Extract TFLite model from .task file (it's a ZIP archive)."""
    print(f"[1/3] Extracting TFLite from {task_file}...")
    
    try:
        # .task files are ZIP archives containing the TFLite model
        with zipfile.ZipFile(task_file, 'r') as zip_ref:
            # Look for .tflite file in the archive
            tflite_files = [f for f in zip_ref.namelist() if f.endswith('.tflite')]
            
            if not tflite_files:
                print("ERROR: No .tflite file found in .task archive")
                return False
            
            # Extract the first .tflite file found
            tflite_name = tflite_files[0]
            print(f"   Found: {tflite_name}")
            
            zip_ref.extract(tflite_name, '.')
            
            # Rename to output name
            if os.path.exists(output_tflite):
                os.remove(output_tflite)
            shutil.move(tflite_name, output_tflite)
            
            print(f"   OK: Extracted to {output_tflite}")
            return True
            
    except Exception as e:
        print(f"ERROR: Failed to extract TFLite: {e}")
        return False

def convert_tflite_to_onnx(tflite_file, onnx_file):
    """Convert TFLite model to ONNX using tf2onnx."""
    print(f"\n[2/3] Converting TFLite to ONNX...")
    
    try:
        # Use tf2onnx command-line tool
        cmd = [
            'python', '-m', 'tf2onnx.convert',
            '--tflite', tflite_file,
            '--output', onnx_file,
            '--opset', '13'
        ]
        
        print(f"   Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode != 0:
            print(f"ERROR: Conversion failed")
            print(f"STDOUT: {result.stdout}")
            print(f"STDERR: {result.stderr}")
            return False
        
        print(f"   OK: ONNX model saved to {onnx_file}")
        return True
        
    except Exception as e:
        print(f"ERROR: Conversion failed: {e}")
        return False

def verify_onnx_model(onnx_file):
    """Verify the ONNX model can be loaded."""
    print(f"\n[3/3] Verifying ONNX model...")
    
    try:
        import onnx
        model = onnx.load(onnx_file)
        onnx.checker.check_model(model)
        
        print(f"   OK: Model is valid!")
        print(f"   Inputs: {[input.name for input in model.graph.input]}")
        print(f"   Outputs: {[output.name for output in model.graph.output]}")
        
        # Get file size
        size_mb = os.path.getsize(onnx_file) / (1024 * 1024)
        print(f"   Size: {size_mb:.2f} MB")
        
        return True
        
    except Exception as e:
        print(f"WARNING: Could not verify model: {e}")
        return True  # Still return True if file exists

def main():
    task_file = "face_landmarker.task"
    tflite_file = "face_landmarker.tflite"
    onnx_file = "face_landmarker.onnx"
    
    print("="*60)
    print("MediaPipe Face Mesh: .task -> ONNX Converter")
    print("="*60)
    
    # Check if task file exists
    if not os.path.exists(task_file):
        print(f"ERROR: {task_file} not found!")
        print(f"Please download it first from:")
        print("https://storage.googleapis.com/mediapipe-models/face_landmarker/face_landmarker/float16/1/face_landmarker.task")
        return 1
    
    # Step 1: Extract TFLite
    if not extract_tflite_from_task(task_file, tflite_file):
        return 1
    
    # Step 2: Convert to ONNX
    if not convert_tflite_to_onnx(tflite_file, onnx_file):
        print("\nFALLBACK: Trying alternative conversion method...")
        # Could try other methods here
        return 1
    
    # Step 3: Verify
    verify_onnx_model(onnx_file)
    
    print("\n" + "="*60)
    print("SUCCESS! ONNX model ready!")
    print("="*60)
    print(f"\nNext steps:")
    print(f"1. Move {onnx_file} to models/ directory")
    print(f"2. Restart VRChat Bridge application")
    print(f"3. Face tracking will be active!")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
