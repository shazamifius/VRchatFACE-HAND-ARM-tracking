"""
Test si ton avatar VRChat peut recevoir les commandes de rotation de tête
Lance ce script PENDANT que tu es dans VRChat avec ton avatar
"""

from pythonosc import udp_client
import time

print("=" * 60)
print("TEST ROTATION TÊTE VRCHAT")
print("=" * 60)
print("\nCe script va envoyer des commandes OSC pour faire bouger ta tête")
print("Regarde ton avatar dans VRChat pendant le test\n")

# Connexion OSC VRChat
client = udp_client.SimpleUDPClient("127.0.0.1", 9000)

print("🔄 Test 1 : HeadPitch (haut/bas)")
print("-" * 40)
for i in range(-10, 11, 2):
    value = i / 10.0
    client.send_message("/avatar/parameters/HeadPitch", value)
    print(f"  Envoyé: HeadPitch = {value:+.1f}")
    time.sleep(0.3)

print("\n🔄 Test 2 : HeadYaw (gauche/droite)")
print("-" * 40)
for i in range(-10, 11, 2):
    value = i / 10.0
    client.send_message("/avatar/parameters/HeadYaw", value)
    print(f"  Envoyé: HeadYaw = {value:+.1f}")
    time.sleep(0.3)

print("\n🔄 Test 3 : HeadRoll (pencher)")
print("-" * 40)
for i in range(-10, 11, 2):
    value = i / 10.0
    client.send_message("/avatar/parameters/HeadRoll", value)
    print(f"  Envoyé: HeadRoll = {value:+.1f}")
    time.sleep(0.3)

# Reset à zéro
client.send_message("/avatar/parameters/HeadPitch", 0.0)
client.send_message("/avatar/parameters/HeadYaw", 0.0)
client.send_message("/avatar/parameters/HeadRoll", 0.0)

print("\n" + "=" * 60)
print("TEST TERMINÉ")
print("=" * 60)
print("\n📋 RÉSULTAT :")
print("- Si ta tête a bougé → Ton avatar supporte HeadPitch/Yaw/Roll ✅")
print("- Si ta tête n'a PAS bougé → Ton avatar n'a pas ces paramètres ❌")
print("\nSi ça n'a pas marché, ton avatar utilise probablement")
print("d'autres noms de paramètres ou n'a pas de head tracking.")
