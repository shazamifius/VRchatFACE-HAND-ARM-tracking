with open(r'c:\Users\shaza\Desktop\avatarVRchatFACEHANDARMtracking\VRchatFACE-HAND-ARM-tracking\src\ui\MainWindow.hpp', 'r', encoding='utf-8') as f:
    content = f.read()

cumulative = 0
for i, line in enumerate(content.split('\n'), 1):
    before = cumulative
    for char in line:
        if char == '{':
            cumulative += 1
        elif char == '}':
            cumulative -= 1
    after = cumulative
    
    # Print lines with brace changes
    if before != after:
        print(f"Line {i}: {before} -> {after} | {line.strip()[:80]}")
        
print(f"\nFinal: {cumulative}")
