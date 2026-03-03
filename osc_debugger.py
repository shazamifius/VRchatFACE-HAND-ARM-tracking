import argparse
import time
import os
import sys
from typing import Any, List

try:
    from pythonosc.dispatcher import Dispatcher
    from pythonosc.osc_server import BlockingOSCUDPServer
except ImportError:
    print("=========================================================")
    print(" ERREUR: Le module 'python-osc' n'est pas installé.")
    print(" Installez-le avec: pip install python-osc")
    print("=========================================================")
    sys.exit(1)

# Configuration interne
IP = "127.0.0.1"
PORT = 9005 # Port "Monitor" utilisé par Rust connectivity.rs
REFRESH_RATE = 1.0 / 30.0 # 30 IPS UI refresh

# Etat de la base de données OSC
osc_data = {}
updates_count = 0
last_ui_update = time.time()

# Layout Categories
categories = {
    "HEAD (Positions & Rotations)": ["HeadPitch", "HeadYaw", "HeadRoll", "HeadPos_X", "HeadPos_Y", "HeadPos_Z"],
    "EYES (Regards & Clignements)": ["EyeBlinkLeft", "EyeBlinkRight", "EyeLookUp", "EyeLookDown", "EyeLookInLeft", "EyeLookInRight"],
    "MOUTH (Machoire & Levres)": ["MouthOpen", "JawOpen", "MouthSmile", "CheekPuff", "MouthPout"],
    "BROWS (Sourcils)": ["BrowInnerUp", "BrowOuterUpLeft", "BrowOuterUpRight", "BrowDownLeft"],
    "HANDS (Pos & Gestes)": ["GestureLeft", "GestureRight", "HandLeftPos_X", "HandRightPos_X"]
}

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def osc_handler(address: str, *args: List[Any]):
    global updates_count
    val = args[0] if len(args) == 1 else args
    osc_data[address] = val
    updates_count += 1

def draw_ui():
    global updates_count
    clear_screen()
    print("=" * 60)
    print(f"📡 VRChat Bridge - OSC Monitor (Ecoute sur {IP}:{PORT})")
    print(f"🔄 Packets recus recents: {updates_count}")
    print("=" * 60)

    # Catégorisation intelligente pour affichage propre
    found_keys = set(osc_data.keys())
    
    for title, keywords in categories.items():
        print(f"\n--- {title} ---")
        category_has_data = False
        
        for k in list(found_keys):
            # Cherche si le chemin OSC /avatar/parameters/X match un mot cle
            if any(kw.lower() in k.lower() for kw in keywords):
                val = osc_data[k]
                
                # Formatage propre
                if isinstance(val, float):
                    val_str = f"{val: >7.3f}"
                    # Bar graph simple pour les floats normalisés
                    if -1.0 <= val <= 1.0:
                        bar_len = 20
                        fill = int(((val + 1.0) / 2.0) * bar_len) if val < 0 else int(val * bar_len)
                        bar = "█" * fill + "░" * (bar_len - fill)
                        print(f"  {k: <35} : {val_str}  [{bar}]")
                    else:
                        print(f"  {k: <35} : {val_str}")
                elif isinstance(val, bool):
                    status = "[X]" if val else "[ ]"
                    print(f"  {k: <35} : {status}")
                else:
                    print(f"  {k: <35} : {val}")
                
                found_keys.remove(k)
                category_has_data = True
                
        if not category_has_data:
            print("  (En attente de donnees...)")
            
    if found_keys:
        print("\n--- AUTRES PARAMETRES OSC ---")
        for k in list(found_keys)[:15]: # Limite pour pas polluer
            val = osc_data[k]
            if isinstance(val, float):
                print(f"  {k: <35} : {val: >7.3f}")
            else:
                 print(f"  {k: <35} : {val}")
        if len(found_keys) > 15:
            print(f"  ... et {len(found_keys) - 15} autres")
            
    print("\n=" * 60)
    print("Pressez Ctrl+C pour quitter.")
    
    updates_count = 0 # Reset pour le compteur "recents"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--ip", default=IP, help="The ip to listen on")
    parser.add_argument("--port", type=int, default=PORT, help="The port to listen on")
    args = parser.parse_args()

    dispatcher = Dispatcher()
    # Intercepte absoluement TOUT
    dispatcher.map("/*", osc_handler)

    server = BlockingOSCUDPServer((args.ip, args.port), dispatcher)
    
    print(f"Demarrage de l'OSC Debugger sur {args.ip}:{args.port}...")
    
    # Run the server in a non-blocking timeout loop to allow UI redraw
    server.timeout = REFRESH_RATE
    try:
        while True:
            server.handle_request()
            now = time.time()
            if now - last_ui_update >= REFRESH_RATE:
                draw_ui()
                last_ui_update = now
    except KeyboardInterrupt:
        print("\nArret du debugger OSC.")
