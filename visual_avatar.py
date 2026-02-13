import argparse
import math
import threading
import time
import tkinter as tk
from pythonosc import dispatcher
from pythonosc import osc_server

# --- STATE ---
# Stores the current value of blendshapes
current_state = {
    "HeadYaw": 0.0, "HeadPitch": 0.0, "HeadRoll": 0.0,
    "EyeOpenLeft": 1.0, "EyeOpenRight": 1.0,
    "EyeSquintLeft": 0.0, "EyeSquintRight": 0.0,
    "MouthSmileLeft": 0.0, "MouthSmileRight": 0.0,
    "MouthFrownLeft": 0.0, "MouthFrownRight": 0.0,
    "JawOpen": 0.0, "MouthFunnel": 0.0, "MouthPucker": 0.0
}
state_lock = threading.Lock()

def osc_handler(address, *args):
    param = address.replace("/avatar/parameters/", "")
    val = args[0]
    
    with state_lock:
        # Map legacy/v2 parameters to unified if needed
        if "FT/v2" in address:
            # Simple mapping for demo
            if "EyeClosed" in param:
                # v2 is closed(1), we want open(1) for consistent drawing logic
                # But our key is Open, so 1 - closed
                side = "Left" if "Left" in param else "Right"
                current_state[f"EyeOpen{side}"] = 1.0 - val
            elif "JawOpen" in param:
                 current_state["JawOpen"] = val
            elif "Smile" in param:
                 current_state[param.replace("FT/v2/", "")] = val
        else:
            # Unified
            current_state[param] = val

class AvatarApp:
    def __init__(self, root):
        self.root = root
        self.root.title("VRChat Face Simulator 🤖")
        self.root.geometry("400x450")
        self.root.configure(bg="#202020")
        
        self.canvas = tk.Canvas(root, width=400, height=400, bg="#202020", highlightthickness=0)
        self.canvas.pack(pady=20)
        
        self.label = tk.Label(root, text="En attente de données OSC...", fg="#00ff00", bg="#202020", font=("Consolas", 10))
        self.label.pack()

        self.update_avatar()

    def draw_eye(self, x, y, width, height, open_factor, squint):
        # Eyelid clipping
        # Draw white sclera
        # Eye height depends on open_factor (0..1)
        # Squint squashes vertically too
        
        h = max(2, height * open_factor * (1.0 - squint * 0.5))
        
        # Draw Eye Background (White)
        self.canvas.create_oval(x - width/2, y - h/2, x + width/2, y + h/2, fill="#ffffff", outline="")
        
        # Draw Pupil (Black) - moves slightly with look (not implemented yet, centered)
        pupil_size = width * 0.3
        if h > pupil_size:
            self.canvas.create_oval(x - pupil_size/2, y - pupil_size/2, 
                                    x + pupil_size/2, y + pupil_size/2, fill="#000000")

    def draw_mouth(self, x, y, width, smile, frown, jaw_open, pucker):
        # Mouth shape calculation
        # Simple quadratic curve
        
        # Width modulation (Pucker makes mouth narrow)
        w = width * (1.0 - pucker * 0.5)
        
        # Corner Y offset
        # Smile goes UP (-), Frown goes DOWN (+)
        corner_y = y - (smile * 15) + (frown * 15)
        
        # Center Y (Jaw Open moves center down)
        center_y = y + (jaw_open * 20)
        
        # Lips points
        p_left = (x - w/2, corner_y)
        p_right = (x + w/2, corner_y)
        p_bottom = (x, center_y + (jaw_open * 10)) # Bottom lip thickness
        
        # Draw open mouth (dark inside) if jaw is open
        if jaw_open > 0.1 or pucker > 0.1:
            # Open shape
            self.canvas.create_polygon(
                x - w/2, corner_y,  # L
                x, center_y - 5,    # Top center
                x + w/2, corner_y,  # R
                x, center_y + 10 + (jaw_open*15), # Bottom
                fill="#400000", smooth=True
            )
        else:
            # Line only
            # Quadratic bezier requires more complex drawing in TK, using Line with smooth=True
            self.canvas.create_line(
                x - w/2, corner_y,
                x, center_y,
                x + w/2, corner_y,
                fill="#ff8888", width=5, capstyle=tk.ROUND, joinstyle=tk.ROUND, smooth=True
            )

    def update_avatar(self):
        self.canvas.delete("all")
        
        with state_lock:
            s = current_state.copy()
            
        # --- HEAD TRANSFORM ---
        # Map -1..1 to pixels
        dx = s.get("HeadYaw", 0) * -50  # Move X
        dy = s.get("HeadPitch", 0) * -40 # Move Y
        # Roll is hard in 2D canvas without rotating everything, skip for now
        
        center_x = 200 + dx
        center_y = 200 + dy
        
        # Draw Face Contour
        self.canvas.create_oval(center_x - 100, center_y - 120, 
                                center_x + 100, center_y + 120, 
                                fill="#ffccaa", outline="#ffaa88", width=3)
        
        # Draw Eyes
        # Left
        self.draw_eye(center_x - 40, center_y - 30, 30, 20, 
                      s.get("EyeOpenLeft", 1.0), s.get("EyeSquintLeft", 0.0))
        # Right
        self.draw_eye(center_x + 40, center_y - 30, 30, 20, 
                      s.get("EyeOpenRight", 1.0), s.get("EyeSquintRight", 0.0))
        
        # Draw Nose (Basic)
        self.canvas.create_oval(center_x - 5, center_y + 10, center_x + 5, center_y + 20, fill="#dda088", outline="")

        # Draw Mouth
        avg_smile = (s.get("MouthSmileLeft", 0) + s.get("MouthSmileRight", 0)) * 0.5
        avg_frown = (s.get("MouthFrownLeft", 0) + s.get("MouthFrownRight", 0)) * 0.5
        
        self.draw_mouth(center_x, center_y + 50, 60, 
                        avg_smile, avg_frown, s.get("JawOpen", 0), s.get("MouthPucker", 0))
        
        # Update text
        status = "Neutre"
        if avg_smile > 0.5: status = "Content ! 😁"
        elif avg_frown > 0.5: status = "Pas content... 😠"
        elif s.get("JawOpen", 0) > 0.5: status = "Parle / Crie 😮"
        
        self.label.config(text=f"Status: {status} | Yaw: {s.get('HeadYaw',0):.2f}")
        
        # Loop at ~30 FPS
        self.root.after(33, self.update_avatar)

# --- OSC SERVER THREAD ---
def start_osc():
    dispatcher_ = dispatcher.Dispatcher()
    dispatcher_.map("/avatar/parameters/*", osc_handler)
    server = osc_server.ThreadingOSCUDPServer(("127.0.0.1", 9000), dispatcher_)
    server.serve_forever()

if __name__ == "__main__":
    # Start OSC
    t = threading.Thread(target=start_osc)
    t.daemon = True
    t.start()
    
    # Start GUI
    root = tk.Tk()
    app = AvatarApp(root)
    root.mainloop()
