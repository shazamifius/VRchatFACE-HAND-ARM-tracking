# VRchat Face & Hand Tracking (Project V2)

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Built%20with-Rust-orange)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Powered%20by-Python%20%2B%20MediaPipe-yellow)](https://www.python.org/)

**Une solution de tracking haute performance pour VRChat, combinant la précision de Google MediaPipe et la rapidité de Rust.**

## 🌟 Fonctionnalités

- **Face Tracking Avancé** : 468 points de visage suivis en temps réel.
- **Hand Tracking** : Détection précise des mains et doigts.
- **Architecture Hybride** :
    - **Backend Python (Invisible)** : Moteur d'IA optimisé (TFLite) tournant en arrière-plan sans interface lourde.
    - **Frontend Rust/Tauri** : Interface utilisateur moderne, légère et fluide.
- **Connexion VRChat** : Support natif du protocole OSC.
- **Support Téléphone** : Utilisez votre smartphone comme webcam via QR Code (Cloudflare Tunnel).

## 🚀 Installation & Utilisation

### Prérequis
- **Windows 10/11**
- **Python 3.10+** (installé et ajouté au PATH)
- **Webcam** (ou Smartphone)

### Démarrage Rapide

1.  **Lancer le Moteur de Tracking** :
    Exécutez `blaze_app_python-main/run_tracker.bat`.
    *Une fenêtre console s'ouvrira pour confirmer le chargement de l'IA. Vous pouvez la réduire.*

2.  **Lancer l'Interface (Hub)** :
    Exécutez `hub/src-tauri/target/release/hub.exe` (ou via Cargo si en mode dev).
    *L'interface détectera automatiquement le tracking et affichera le retour vidéo.*

3.  **VRChat** :
    Activez l'OSC dans le menu radial de VRChat. Le logiciel enverra automatiquement les données sur le port `9000`.

## 🛠️ Architecture Technique

Le projet utilise une approche **micro-services locale** pour maximiser les performances :

| Composant | Technologie | Rôle |
|:---:|:---:|:---|
| **Tracker** | Python, MediaPipe, TFLite | Capture Webcam, Inférence IA, Envoi OSC (Port 9002), Stream MJPEG (Port 8080) |
| **Hub** | Rust, Tauri, React | Interface Utilisateur, Réception OSC (9002), Solver VRChat, Envoi OSC Final (9000) |
| **Connectivity** | Cloudflared, Axum | Gestion du tunnel sécurisé pour connecter un téléphone distant |

## 📦 Structure du Projet

- `blaze_app_python-main/` : Le cerveau IA (Python). Contient le script `tracker_headless.py`.
- `hub/` : Le corps (Rust + Tauri). Contient l'application desktop.
- `scripts/` : Utilitaires divers.

## 🤝 Crédits

- Basé sur les travaux de Google MediaPipe.
- Inspiré par [AlbertaBeef/blaze_app_python](https://github.com/AlbertaBeef/blaze_app_python).
- Développé pour la communauté VRChat.

## 📄 Licence

Apache 2.0
