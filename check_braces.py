with open(r'c:\Users\shaza\Desktop\avatarVRchatFACEHANDARMtracking\VRchatFACE-HAND-ARM-tracking\src\ui\MainWindow.hpp', 'r', encoding='utf-8') as f:
    content = f.read()
    
opens = content.count('{')
closes = content.count('}')

print(f"Opening braces: {opens}")
print(f"Closing braces: {closes}")
print(f"Difference: {opens - closes}")

# Print line by line with cumulative count
cumulative = 0
for i, line in enumerate(content.split('\n'), 1):
    for char in line:
        if char == '{':
            cumulative += 1
        elif char == '}':
            cumulative -= 1
    if cumulative < 0:
        print(f"Line {i}: Cumulative goes negative! {cumulative}")
        print(f"  Content: {line}")
        break
        
print(f"\nFinal cumulative: {cumulative}")
