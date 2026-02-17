# Avancement du Projet - État au 16/02/2026

Ce document recense l'intégralité des actions effectuées sur le projet et documente son état technique précis.

## 🎯 Rappel de l'Objectif

"L'objectif de ce projet est d'offrir aux utilisateurs PC de VRChat, un moyen d'avoir un très bon face tracking de qualité ainsi qu'un hand+arm tracking lorsque ceux-ci apparaissent à la caméra uniquement.
Il faut que ce soit d'une grande précision dans l'espace ainsi que savoir précisément la position de la main. Et tout ça, il faut le reproduire directement sur les avatars de VRChat avec la plus grande précision, la meilleure qualité et la plus grande rapidité possible."

---

## 🏗️ État des Lieux Technique : Architecture Native (Rust V2) ✅

Après avoir testé une architecture hybride (Python + Rust), nous avons finalement opté pour une **Architecture 100% Native Rust**.

### Pourquoi ce changement ?

1. **Performance** : Le moteur Python (MediaPipe) était précis mais lourd (CPU usage élevé, latence). La version Rust utilise les mêmes modèles mathématiques (BlazeFace/BlazeLandmark) via `ONNX Runtime` mais avec une performance **x10**.
2. **Stabilité** : Plus de dépendance à l'installation de Python, de `pip`, ou de conflits de versions. Tout est dans un seul exécutable `.exe`.
3. **Support Téléphone** : La latence réseau est minimisée grâce au traitement natif, permettant un tracking fluide même via Wi-Fi.

---

## 🛠️ Actions Effectuées & Validées

### 1. Moteur de Tracking V2 (Rust)

* **Inférence ONNX Native** :
  * Utilisation de `ort` (ONNX Runtime) directement dans le backend Tauri.
  * Chargement des modèles `blaze_face_short_range.onnx` et `blaze_landmark.onnx`.
  * **Fix Critique** : Correction de l'extraction des landmarks qui étaient ignorés dans les premières versions.
* **Optimisation** :
  * Multithreading : Capture vidéo et Inférence tournent sur des threads séparés pour ne jamais bloquer l'UI.
  * Zéro-Copie (ou presque) sur le traitement d'image.

### 2. Support Téléphone ("Phone Camera")

* **Mode "Scan & Play"** :
  * L'utilisateur scanne un QR code sur l'interface PC.
  * Le téléphone (iOS/Android) devient instantanément une webcam HD sans installer d'application (via navigateur WebRTC/MJPEG).
* **Tunneling Cloudflare** :
  * Intégration d'un téléchargement et lancement automatique de `cloudflared` pour permettre la connexion même si le pare-feu est strict.
* **Correction Scintillement** :
  * Correction d'un bug où des "frames vides" faisaient clignoter l'interface.
  * Stabilisation CSS pour éviter que la vidéo ne redimensionne l'interface.

### 3. Expérience Utilisateur (UX)

* **Launcher Unifié (`START_PROJECT.bat`)** :
  * Vérifie automatiquement la présence de Rust et Cloudflare.
  * Installe les dépendances manquantes sans ligne de commande compliquée.
  * Interface couleur "User Friendly".
* **Interface Premium** :
  * Thème "Dark Glass" moderne.
  * Indicateurs visuels (Visage détecté, Mains détectées, FPS, Latence).

---

## 🚧 Ce qu'il Reste à Faire (Roadmap)

1. **Affinage du Solver VRChat** :
    * Le moteur détecte le visage, maintenant il faut mapper les 468 points vers les paramètres VRChat (JawOpen, EyeBlink, etc.) avec plus de subtilité.
2. **Tracking des Mains et Bras avec IK** :
    * Intégrer pleinement le modèle de mains (MediaPipe Hands) dans le pipeline Rust (actuellement en cours de portage complet).
    * Calculer la position des coudes (Arm Tracking) par cinématique inverse (IK).
3. **Tests Grande Échelle** :
    * Valider la stabilité sur des sessions de plusieurs heures.

---

## 📅 Synthèse

Nous avons pivoté d'une solution "Bricolage Python" vers une véritable **Application Desktop Native**.
C'est plus rapide, plus stable, et prêt pour le grand public.

### État Actuel : 🟢 Fonctionnel & Stable (Face Tracking de base + Vidéo Fluide)
