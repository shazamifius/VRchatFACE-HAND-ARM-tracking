# 📊 État du Projet - VRChat Universal Video Bridge

**Date**: 2026-01-28
**Version Actuelle**: 0.3.2 (Alpha - Phone Link Implementation)
**Objectif Final**: Leader 2026 - Face + Hand + Arm Tracking Complet

---

## 🎯 Vision du Projet (d'après TECHNICAL_PLAN.md)

### Objectif Final

Application **portable, zéro-dépendance** pour tracking **Face + Hands + Arms** via webcam ou téléphone, avec intégration native VRChat (OSC).

### Promesses Clés

1. ✅ Installation en 1 clic (.zip portable)
2. ✅ Auto-configuration VRChat (OSC automatique)
3. ⏳ **Tracking complet**: Visage + Mains + Bras
4. ✅ Adaptation automatique de qualité (Ultra/Balanced/Eco)
5. ✅ Connexion téléphone via QR Code (HTTP POST)
6. ✅ Performance 60 FPS même sur iGPU

---

## ✅ Ce qui est IMPLÉMENTÉ

### 1. **Infrastructure de Base** ✅

| Composant | Status | Détails |
|-----------|--------|---------|
| CMake + vcpkg | ✅ Complet | Build system fonctionnel |
| Architecture multithread | ✅ Complet | VisionLoop dans thread séparé |
| ONNX Runtime + DirectML | ✅ Complet | Inférence GPU fonctionnelle |
| OpenCV | ✅ Complet | Capture webcam multiple caméras |
| ImGui UI | ✅ Complet | Interface utilisateur complète |
| OSC Client | ✅ Complet | Envoi vers VRChat fonctionnel |
| Web Server (Phone Link) | ✅ Complet | HTTP POST endpoint pour caméra mobile |

### 2. **Vision AI** ✅

| Feature | Status | Détails |
|---------|--------|---------|
| YOLOv8-Pose | ✅ Implémenté | 17 keypoints COCO détectés |
| Post-processing | ✅ Implémenté | Extraction keypoints fonctionnelle |
| Détection multiple caméras | ✅ Implémenté | Sélection avec indicateurs de statut |
| Sélecteur adaptatif QualityMode | ✅ Implémenté | Ultra/Balanced/Eco automatique |

## 🔄 En Cours d'Intégration (Phase Cleanup & AI)

### Nouveaux Modèles Identifiés (Source: User)

Nous avons identifié les dépôts exacts pour les modèles manquants:

1. **Face Tracking**: `Facial-Landmark-Detection.onnx` (Qualcomm) - *Status: A intégrer*
2. **Hand Tracking**: `MediaPipeHandDetector.onnx` (Qualcomm) - *Status: A intégrer*
3. **Pose Tracking**: `yolov8n-pose.onnx` (Déjà présent)

Ceci remplace la nécessité de convertir manuellement les fichiers `.task` de MediaPipe.

### 3. **Visualisation 3D** ✅

| Feature | Status | Détails |
|---------|--------|---------|
| Squelette 3D OpenGL | ✅ Implémenté | Rendu temps réel |
| Grille de référence | ✅ Implémenté | Grille au sol |
| Contrôles caméra 3D | ✅ Implémenté | WASDQER pour navigation |
| Mode Test (sliders UI) | ✅ Implémenté | Debug manuel |

### 4. **UI & UX** ✅

| Feature | Status | Détails |
|---------|--------|---------|
| Compteurs FPS/Latency | ✅ Implémenté | Monitoring temps réel |
| Sélection caméra améliorée | ✅ Implémenté | Indicateurs vert/rouge |
| Preview caméra | ✅ Implémenté | Flux vidéo affiché |
| Preview squelette | ✅ Implémenté | Visualisation 3D |

---

## ❌ Ce qui MANQUE (Par rapport au plan)

### 1. **Tracking Facial Complet** ❌ **CRITIQUE**

| Feature | Status | Impact |
|---------|--------|--------|
| 468 face landmarks | ⏳ En cours | Intégration `Facial-Landmark-Detection.onnx` requise |
| Blendshapes faciaux | ❌ Manquant | Pas d'expressions |
| Jaw open (bouche) | ❌ Manquant | Pas de parole |
| Eye blink L/R | ❌ Manquant | Visage statique |

### 2. **Hand Tracking** ❌ **CRITIQUE**

| Feature | Status | Impact |
|---------|--------|--------|
| Hand Detection | ⏳ En cours | Intégration `MediaPipeHandDetector.onnx` requise |
| Finger tracking | ❌ Manquant | Pas de gestures |

---

## 🗺️ Roadmap Mise à Jour (Cleanup Phase)

### Phase 1: Nettoyage et Téléchargement des Modèles 🔴 IMMÉDIAT

- [x] Analyser `scripts/` et `src/`
- [ ] Mettre à jour `download_models.bat` avec les nouveaux liens directs
- [ ] Supprimer les scripts de conversion obsolètes (`convert_task_to_onnx.py`) ? -> *A garder comme utilitaire, mais pas critique.*

### Phase 2: Intégration Face & Hand Models

- [ ] Modifier `VisionSystem` pour charger `Facial-Landmark-Detection.onnx`
- [ ] Modifier `VisionSystem` pour charger `MediaPipeHandDetector.onnx`
- [ ] Vérifier les outputs (tensor shapes)

### Phase 3: Post-Processing

- [ ] Convertir les landmarks Face -> Blendshapes
- [ ] Convertir les landmarks Hand -> Bone Rotations

### Phase 4: Phone Link Cloud Distribution ⏳ EN COURS

- [x] Implémentation serveur HTTP POST pour caméra mobile
- [x] Correction dépendance QR code (libqrencode)
- [ ] Hébergement site web sur GitHub Pages
- [ ] Intégration Cloudflare Tunnel pour accès mondial
- [ ] Génération QR code dynamique avec URL publique

**Architecture proposée** :

- Site web hébergé : `https://shazamifius.github.io/VRchatFACE-HAND-ARM-tracking`
- Tunnel automatique : Cloudflare Quick Tunnel (gratuit, pas de compte)
- QR code : URL GitHub + paramètre tunnel pour routage

**Corrections récentes** :

- ✅ Dépendance vcpkg : `qrcodegen` (invalide) → `libqrencode`
- ✅ API adaptation : qrcodegen C++ → libqrencode C
- ✅ Serveur Phone Link : WebSocket → HTTP POST (plus simple et compatible)
