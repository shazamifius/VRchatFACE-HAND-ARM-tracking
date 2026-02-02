#!/usr/bin/env python3
"""
Quick fix script to replace Send() with SendFloat() in main.cpp for OSC blendshapes
"""
import re

file_path = r"c:\Users\Administrateur\Desktop\VRchatFACE-HAND-AR

M-tracking\src\main.cpp"

with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace all occurrences of osc_client.Send( with osc_client.SendFloat(
# But ONLY in thesection between lines 405-425 (phone blendshapes section)
lines = content.split('\n')
modified = []
in_target_section = False

for i, line in enumerate(lines, 1):
    # Detect start of target section
    if '// Eyes' in line and i >= 400 and i <= 410:
        in_target_section = True
    
    # Detect end of target section  
    if in_target_section and ('// 6. Send OSC - Skeleton' in line or 'for (const auto &bone : skeleton_pose)' in line):
        in_target_section = False
    
    # Replace in target section only
    if in_target_section and 'osc_client.Send(' in line:
        line = line.replace('osc_client.Send(', 'osc_client.SendFloat(')
    
    modified.append(line)

# Write back
with open(file_path, 'w', encoding='utf-8', newline='\r\n') as f:
    f.write('\n'.join(modified))

print("[✓] Replaced Send() with SendFloat() in phone blendshapes section")
