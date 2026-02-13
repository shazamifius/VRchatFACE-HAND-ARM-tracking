# Avancement du Projet - État au 13/02/2026

Ce document recense l'intégralité des actions effectuées sur le projet et documente son état technique précis.

## 🎯 Rappel de l'Objectif

"L'objectif de ce projet est d'offrir aux utilisateurs PC de VRChat, un moyen d'avoir un très bon face tracking de qualité ainsi qu'un hand+arm tracking lorsque ceux-ci apparaissent à la caméra uniquement.
Il faut que ce soit d'une grande précision dans l'espace ainsi que savoir précisément la position de la main. Et tout ça, il faut le reproduire directement sur les avatars de VRChat avec la plus grande précision, la meilleure qualité et la plus grande rapidité possible."

---

## 🏗️ État des Lieux Technique : Architecture Hybride (Validée ✅)

Après une phase d'expérimentation complexe sur la conversion de modèles ONNX (instables sous Windows), nous avons pivoté vers une **architecture hybride ultra-performante** qui combine le meilleur des deux mondes :

1.  **Cerveau (Python Headless)** :
    *   Utilise l'infrastructure Google MediaPipe/Blaze native (TFLite) pour une précision maximale.
    *   Tourne en mode "invisible" (sans fenêtre GUI) pour économiser le GPU.
    *   Envoie les données de tracking (468 points visage + 21 points main) via **OSC rapide** (UDP Local).
    *   Diffuse un retour vidéo optimisé (squelette fil de fer) via un serveur **MJPEG ultra-léger**.

2.  **Corps (Rust + Tauri)** :
    *   Interface utilisateur moderne, fluide et "Premium".
    *   Affiche le retour vidéo du cerveau Python sans latence.
    *   Se chargera de la logique métier complexe (Solver VRChat, lissage OneEuroFilter, paramètres d'avatar).

### Pourquoi ce choix ?
*   **Précision Absolue** : On ne réinvente pas la roue. On utilise le moteur d'inférence original de Google qui est "Pixel Perfect".
*   **Performance** : Python ne fait que du calcul pur (C++ wrappers). Rust ne fait que de l'affichage et de la logique légère. Aucun gaspillage de ressources.
*   **Robustesse** : Fini les problèmes de conversion ONNX ou de compatibilité de drivers.

---

## 🛠️ Actions Effectuées : Détail Précis

### 1. Moteur de Tracking (`tracker_headless.py`)
*   **Implémentation d'un Tracker "Invisible"** :
    *   Basé sur `blaze_app_python`.
    *   Suppression de toutes les dépendances graphiques lourdes (OpenCV HighGUI).
    *   Correction automatique des incompatibilités `Numpy 2.0` vs legacy libraries.
*   **Serveur MJPEG Multithreadé** :
    *   Diffusion du flux vidéo de tracking sur `http://localhost:8080`.
    *   Support du "Time-travel debugging" (via paramètres URL).
*   **Output OSC** :
    *   Envoi temps réel des landmarks bruts sur le port `9002`.

### 2. Interface Hub (Rust/Tauri)
*   **Intégration Vidéo** :
    *   Le Hub Rust consomme et affiche le flux MJPEG avec une latence quasi-nulle.
    *   Gestion automatique des erreurs de connexion et reconnexion.
*   **Design & UX** :
    *   Interface propre, indicateurs de statut (Face/Hand/FPS) connectés au moteur.

---

## 🚧 Ce qu'il Reste à Faire (Prochaines Étapes)

Le système "voit" parfaitement (Python) et "montre" ce qu'il voit (Rust). Il reste à "traduire" cette vision pour VRChat.

1.  **Le "Solver" (Rust)** :
    *   Le Hub Rust doit écouter le port `9002` (OSC) pour recevoir les points bruts.
    *   Il doit transformer ces points mathématiques en paramètres VRChat (ex: "JawOpen", "EyeBlinkLeft", "HeadYaw"...).
2.  **Stabilisation (Anti-Tremblement)** :
    *   Implémenter le filtre **OneEuro** dans Rust pour lisser les micro-tremblements inévitables de la webcam, surtout pour les mains.
3.  **Envoi VRChat** :
    *   Connecter la sortie du Solver vers le port `9000` (VRChat).

## 📅 Synthèse

Nous avons réussi à :
1.  Contourner les limitations techniques de Windows/ONNX.
2.  Obtenir un tracking fluide et précis.
3.  Avoir une interface utilisateur professionnelle qui masque la complexité technique.

**Le projet est sur des rails solides pour la phase finale : l'intégration VRChat.**
