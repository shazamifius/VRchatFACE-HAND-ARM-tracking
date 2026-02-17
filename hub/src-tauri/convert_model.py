
import os
import sys
import subprocess

def install(package):
    subprocess.check_call([sys.executable, "-m", "pip", "install", package])

def convert():
    print("Checking dependencies...")
    try:
        import tf2onnx
    except ImportError:
        print("Installing tf2onnx...")
        install("tf2onnx")

    try:
        import tensorflow
    except ImportError:
        print("Installing tensorflow (cpu)...")
        install("tensorflow-cpu")


    models_to_convert = [
        "face_detection_short_range",
        "face_landmark",
        "palm_detection_lite",
        "hand_landmark_lite" 
    ]

    for model_name in models_to_convert:
        model_path = os.path.join("models", f"{model_name}.tflite")
        output_path = os.path.join("models", f"{model_name}.onnx")

        if not os.path.exists(model_path):
            print(f"Error: {model_path} not found!")
            continue

        if os.path.exists(output_path):
             print(f"Skipping {output_path} (already exists)")
             continue

        print(f"Converting {model_path} to {output_path}...")
        
        # Run tf2onnx
        cmd = [
            sys.executable, "-m", "tf2onnx.convert",
            "--tflite", model_path,
            "--output", output_path,
            "--opset", "11"
        ]
        
        try:
            subprocess.check_call(cmd)
            print(f"Successfully converted {model_name}")
        except subprocess.CalledProcessError as e:
            print(f"Failed to convert {model_name}: {e}")

    print("All conversions attempted!")

if __name__ == "__main__":
    convert()
