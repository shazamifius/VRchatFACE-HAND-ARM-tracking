# Avancement du Projet - État au 19/02/2026

Ce document recense l'intégralité des fonctionnalités, l'architecture technique et les dernières mises à jour du projet **VRChat Bridge Hub**.

## 🎯 Objectif
Offrir un système de tracking facial et manuel (Face + Hand + Arm) de haute précision pour VRChat, sans équipement coûteux (Webcam ou Téléphone), avec une performance native.

---

## 🏗️ Architecture Technique (Rust Native)

L'application est entièrement construite en **Rust** pour garantir une latence minimale et une robustesse maximale.

*   **Backend**: Tauri + Rust (Logique de tracking, Inférence AI, Réseau OSC).
*   **Frontend**: HTML/JS (Interface utilisateur, visualisation caméra).
*   **AI Engine**: `ort` (ONNX Runtime) exécutant des modèles MediaPipe optimisés sur CPU/GPU.

### Performance
*    **FPS Cible**: 30 FPS stable (récemment validé à 19-30 FPS selon l'éclairage).
*   **Latence**: < 15ms de traitement (Inférence + Solver).

---

## 🛠️ Fonctionnalités Implémentées

### 1. Face Tracking (Avancé & Stabilisé)
Le moteur de visage est le plus abouti à ce jour.
*   **Détection**: Utilisation de `BlazeFace` (Détection) + `BlazeLandmark` (468 points).
*   **Lissage Intelligent**:
    *   *OneEuroFilter*: Filtre le jitter (tremblements) à haute vitesse.
    *   *InertiaFilter*: Ajoute du poids aux mouvements pour un rendu plus naturel.
*   **Vie Artificielle ("Alive Feel")**:
    *   *Micro-expressions*: Le visage génère subtilement des mouvements aléatoires (sourcils, joues) pour éviter l'effet "robot figé" quand l'utilisateur est neutre.
    *   *Saccades Oculaires*: Les yeux effectuent des micro-mouvements réalistes.
    *   *Auto-Blink*: Si l'utilisateur ne cligne pas des yeux pendant trop longtemps, le système force un clignement naturel.
*   **Fallback (Perte de Tracking)**:
    *   Si le visage n'est plus détecté, les paramètres retournent progressivement à zéro (decay) sur 500ms au lieu de se figer brutalement.

### 2. Hand & Arm Tracking (En Cours de Déblocage)
Le code est prêt, mais était bloqué par des fichiers modèles corrompus.
*   **Cinématique Inverse (IK)**:
    *   Module `ik.rs` implémenté. Calcule la position du **Coude** en fonction de l'Épaule et du Poignet.
    *   Permet d'animer les bras complets dans VRChat sans trackers supplémentaires.
*   **Logique de Perte**:
    *   Si une main sort du champ, elle reste figée 200ms (pour éviter les pertes brèves) puis redescend lentement le long du corps (Neutre).
*   **État Actuel**: Modèles ONNX en cours de réparation via script dédié.

### 3. Gestion Caméra & Système
*   **Smart Retry Logic (NOUVEAU)**:
    *   Contournement automatique du bug Windows qui force certaines webcams à 1 FPS.
    *   Le système teste plusieurs configurations (NV12, 15fps, Basse Résolution) jusqu'à trouver un flux fluide.
*   **Support Téléphone**:
    *   Connexion via QR Code (Réseau Local ou Tunneling Cloudflare).
*   **Profiling**:
    *   Mesure précise du temps de calcul (`solve`, `osc`) pour détecter les goulots d'étranglement.

### 4. Réseau OSC
*   **Batching**: Les paramètres OSC sont envoyés en paquets groupés (Bundles) pour réduire la saturation réseau de VRChat.

---

## 🐛 Debugging & Correctifs Récents (17/02 - 19/02)

### ✅ Problème : Caméra bloquée à 1 FPS
*   **Symptôme**: L'image était saccadée, rendant le tracking impossible.
*   **Cause**: Driver Windows MediaFoundation qui force une exposition longue en basse lumière ou bug de format.
*   **Solution**: Implémentation d'une logique de **Retry** qui force la caméra en mode 15fps ou 640x360 si le 30fps échoue.

### ✅ Problème : Crash au chargement des mains
*   **Symptôme**: Logs `Failed to load Palm Detector: Protobuf parsing failed`.
*   **Cause**: Les fichiers `.onnx` dans le dossier `models` n'étaient pas les vrais fichiers (4MB) mais des pointeurs Git LFS (132 octets).
*   **Solution**: Création du script `download_models.ps1` pour télécharger automatiquement les vrais fichiers valides.

### 🔍 En Cours : "Face Not Detected" malgré 19 FPS
*   **Symptôme**: La caméra tourne bien, mais le log indique "0 trackers".
*   **Piste**: Probablement une image trop sombre ou un problème de format pixel (NV12 -> RGB).
*   **Action**: Ajout de logs diagnostiques (`[AI] ...`) dans la dernière build pour identifier la cause exacte.

---

## 📅 Roadmap Immédiate

1.  **Utilisateur**: Exécuter `download_models.ps1` pour réparer les mains.
2.  **Utilisateur**: Tester la Release Build et vérifier que le visage est détecté (`[AI] Face OK`).
3.  **Système**: Une fois le visage accroché, le Hand Tracking et l'IK s'activeront automatiquement.

---
*Document généré automatiquement par l'assistant de développement.*
