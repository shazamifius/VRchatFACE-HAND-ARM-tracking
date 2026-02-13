
import sys
import os
import time

print("[DEBUG] Script launched. Initializing imports...")

try:
    import numpy as np
    # [FIX] Numpy 2.0 compatibility patch
    try:
        np.object = object
    except AttributeError:
        pass
    print("[DEBUG] Numpy imported successfully.")
except ImportError as e:
    print(f"[ERROR] Critical: Numpy is missing or broken. {e}")
    input("Press Enter to exit...")
    sys.exit(1)
except Exception as e:
    print(f"[ERROR] Critical: Numpy crash. {e}")
    input("Press Enter to exit...")
    sys.exit(1)

import argparse
import cv2
import socket
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from socketserver import ThreadingMixIn

# Add local modules to path
sys.path.append(os.path.abspath('blaze_common/'))
sys.path.append(os.path.abspath('blaze_tflite/'))

# OSC Import (Try/Except to be safe)
try:
    from pythonosc import udp_client
    HAS_OSC = True
except ImportError:
    print("[WARN] python-osc not found. OSC sending disabled.")
    HAS_OSC = False

# Import Blaze Modules (Hardcoded for stability with TFLite)
print("[DEBUG] Importing Blaze modules...")
try:
    from blaze_tflite.blazedetector import BlazeDetector
    from blaze_tflite.blazelandmark import BlazeLandmark
    from visualization import draw_detections, draw_landmarks, draw_roi
    from visualization import HAND_CONNECTIONS, FACE_CONNECTIONS
    print("[DEBUG] Blaze modules imported successfully.")
except ImportError as e:
    print(f"[ERROR] Failed to import Blaze modules: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)
except Exception as e:
    print(f"[ERROR] Unexpected error during imports: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)

# --- CONFIGURATION ---
MJPEG_PORT = 8080
OSC_IP = "127.0.0.1"
OSC_PORT = 9002
VIDEO_WIDTH = 640
VIDEO_HEIGHT = 480

# Globals for frame sharing
output_frame = None
lock = threading.Lock()

# --- MJPEG SERVER ---
class MJPEGHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        global output_frame, lock
        # [FIX] Accept root path even with query parameters (e.g. /?t=123)
        if self.path.startswith('/'):
            self.send_response(200)
            self.send_header('Content-type', 'multipart/x-mixed-replace; boundary=frame')
            self.end_headers()
            while True:
                with lock:
                    if output_frame is None:
                        time.sleep(0.01)
                        continue
                    (flag, encodedImage) = cv2.imencode(".jpg", output_frame)
                    if not flag:
                        continue
                
                try:
                    self.wfile.write(b'--frame\r\n')
                    self.send_header('Content-Type', 'image/jpeg')
                    self.end_headers()
                    self.wfile.write(bytearray(encodedImage))
                    self.wfile.write(b'\r\n')
                except ConnectionResetError:
                    break
                except Exception:
                    break
                time.sleep(0.01) # Max 100 FPS loop
        else:
            self.send_error(404)

class ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
    pass

def start_mjpeg_server():
    try:
        server = ThreadingHTTPServer(('0.0.0.0', MJPEG_PORT), MJPEGHandler)
        print(f"[INFO] MJPEG Server started on port {MJPEG_PORT}")
        server.serve_forever()
    except Exception as e:
        print(f"[ERROR] MJPEG Server failed: {e}")

# --- MAIN TRACKER ---
def main():
    print("[DEBUG] Starting main loop...")
    global output_frame, lock

    # 1. Setup OSC
    osc_client = None
    if HAS_OSC:
        osc_client = udp_client.SimpleUDPClient(OSC_IP, OSC_PORT)
        print(f"[INFO] OSC Sender ready on {OSC_IP}:{OSC_PORT}")

    # 2. Setup Camera
    print("[DEBUG] Opening Camera...")
    cap = cv2.VideoCapture(0)
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, VIDEO_WIDTH)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, VIDEO_HEIGHT)
    
    if not cap.isOpened():
        print("[ERROR] Could not open webcam.")
        return
    print("[DEBUG] Camera opened.")

    # 3. Setup Models (Face & Hand)
    # Using Short Range Face & Lite Hand for performance
    print("[INFO] Loading Models...")
    
    try:
        # Face
        print("[DEBUG] Loading Face Detector...")
        face_detector = BlazeDetector("blazeface")
        face_detector.load_model("blaze_tflite/models/face_detection_short_range.tflite")
        
        print("[DEBUG] Loading Face Landmark...")
        face_landmark = BlazeLandmark("blazefacelandmark")
        face_landmark.load_model("blaze_tflite/models/face_landmark.tflite")
        
        # Hand
        print("[DEBUG] Loading Hand Detector...")
        palm_detector = BlazeDetector("blazepalm")
        palm_detector.load_model("blaze_tflite/models/palm_detection_lite.tflite")
        
        print("[DEBUG] Loading Hand Landmark...")
        hand_landmark = BlazeLandmark("blazehandlandmark")
        hand_landmark.load_model("blaze_tflite/models/hand_landmark_lite.tflite")
    except Exception as e:
        print(f"[ERROR] Failed to load models: {e}")
        import traceback
        traceback.print_exc()
        return

    print("[INFO] Models Loaded. Starting Loop...")

    # Start MJPEG Thread
    t = threading.Thread(target=start_mjpeg_server, daemon=True)
    t.start()

    while True:
        ret, frame = cap.read()
        if not ret:
            print("[WARN] Frame capture failed")
            time.sleep(0.1)
            continue

        # Flip for mirror effect
        frame = cv2.flip(frame, 1)
        
        # Prepare for inference
        img_rgb = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        
        # --- FACE TRACKING ---
        img_face, scale_face, pad_face = face_detector.resize_pad(img_rgb)
        norm_dets_face = face_detector.predict_on_image(img_face)
        
        if len(norm_dets_face) > 0:
            dets_face = face_detector.denormalize_detections(norm_dets_face, scale_face, pad_face)
            xc, yc, scale, theta = face_detector.detection2roi(dets_face)
            roi_img, roi_affine, _ = face_landmark.extract_roi(img_rgb, xc, yc, theta, scale)
            
            flags, norm_lms_face = face_landmark.predict(roi_img)
            lms_face = face_landmark.denormalize_landmarks(norm_lms_face, roi_affine)
            
            # Draw
            for i in range(len(flags)):
                if flags[i] > 0.5:
                    draw_landmarks(frame, lms_face[i][:,:2], FACE_CONNECTIONS, thickness=1, radius=1, color=(0, 255, 255))
                    
                    # Send OSC (Face)
                    if osc_client:
                        # Flatten landmarks: x1,y1,z1, x2,y2,z2...
                        flat_lms = lms_face[i].flatten().tolist()
                        osc_client.send_message("/tracking/face/landmarks", flat_lms)

        # --- HAND TRACKING ---
        img_hand, scale_hand, pad_hand = palm_detector.resize_pad(img_rgb)
        norm_dets_hand = palm_detector.predict_on_image(img_hand)

        if len(norm_dets_hand) > 0:
            dets_hand = palm_detector.denormalize_detections(norm_dets_hand, scale_hand, pad_hand)
            xc, yc, scale, theta = palm_detector.detection2roi(dets_hand)
            roi_img, roi_affine, _ = hand_landmark.extract_roi(img_rgb, xc, yc, theta, scale)
            
            # Predict returns different tuple size for hands sometimes (flags, lms, handedness)
            res = hand_landmark.predict(roi_img)
            if len(res) == 3:
                flags, norm_lms_hand, handedness = res
            else:
                flags, norm_lms_hand = res
                handedness = [0.0] * len(flags) # Dummy

            lms_hand = hand_landmark.denormalize_landmarks(norm_lms_hand, roi_affine)

            # Draw
            for i in range(len(flags)):
                if flags[i] > 0.5:
                     draw_landmarks(frame, lms_hand[i][:,:2], HAND_CONNECTIONS, thickness=2, radius=2, color=(0, 255, 0))
                     
                     # Send OSC (Hand)
                     if osc_client:
                         flat_lms = lms_hand[i].flatten().tolist()
                         osc_client.send_message(f"/tracking/hand/{i}/landmarks", flat_lms)

        # Update MJPEG Frame
        with lock:
            output_frame = frame.copy()
        
        # FPS Control (optional, prevent CPU burn)
        # time.sleep(0.001)

if __name__ == "__main__":
    main()
