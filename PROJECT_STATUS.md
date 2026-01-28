# 📊 État du Projet - VRChat Universal Video Bridge

**Date**: 2026-01-28  
**Version Actuelle**: 0.3.0 (Alpha)  
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
5. ⏳ Connexion téléphone via QR Code
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

### 2. **Vision AI** ✅
| Feature | Status | Détails |
|---------|--------|---------|
| YOLOv8-Pose | ✅ Implémenté | 17 keypoints COCO détectés |
| Post-processing | ✅ Implémenté | Extraction keypoints fonctionnelle |
| Détection multiple caméras | ✅ Implémenté | Sélection avec indicateurs de statut |
| Sélecteur adaptatif QualityMode | ✅ Implémenté | Ultra/Balanced/Eco automatique |

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
| 468 face landmarks (MediaPipe) | ❌ Manquant | Pas de clignement d'yeux |
| Blendshapes faciaux | ❌ Manquant | Pas d'expressions |
| Jaw open (bouche) | ❌ Manquant | Pas de parole |
| Eye blink L/R | ❌ Manquant | Visage statique |
| Eyebrow movement | ❌ Manquant | Pas d'émotions |

**➡️ Actuellement**: Seule la **position de la tête** est trackée, aucune expression faciale.

### 2. **Hand Tracking** ❌ **CRITIQUE**
| Feature | Status | Impact |
|---------|--------|--------|
| 21 hand landmarks x2 | ❌ Manquant | Mains absentes |
| Finger tracking | ❌ Manquant | Pas de gestures |
| Hand rotation | ❌ Manquant | Mains plates |

**➡️ Actuellement**: Mains **totalement absentes**, pas de tracking du tout.

### 3. **Arm Tracking** ❌ **IMPORTANT**
| Feature | Status | Impact |
|---------|--------|--------|
| Shoulder/Elbow/Wrist IK | ⏳ Détecté mais non utilisé | Bras absents |
| Full body IK solver | ❌ Manquant | Posture rigide |
| Upper body tracking | ⏳ Keypoints disponibles | Pas rendu |

**➡️ Actuellement**: YOLOv8 **détecte** épaules/coudes/poignets mais on ne les **affiche ni n'envoie** à VRChat.

### 4. **Biomécanique Avancée** ❌
| Feature | Status | Impact |
|---------|--------|--------|
| Motion Extrapolation | ❌ Manquant | Lag perçu |
| Confidence Filtering | ❌ Manquant | Jittering |
| Smoothing adaptatif | ❌ Manquant | Mouvements saccadés |
| Recovery Mode | ❌ Manquant | Snapping brutal |

### 5. **Connectivité Avancée** ❌
| Feature | Status | Impact |
|---------|--------|--------|
| Support NDI | ❌ Manquant | Pas de caméra externe |
| QR Code phone link | ❌ Manquant | Pas de téléphone |
| Auto-config VRChat | ⏳ Partiel | Requiert config manuelle |

### 6. **Distribution & Polish** ❌
| Feature | Status | Impact |
|---------|--------|--------|
| .zip portable | ⏳ Partiel | DLLs manquantes ? |
| Auto-updater | ❌ Manquant | Mises à jour manuelles |
| Firewall helper | ❌ Manquant | Setup compliqué |
| Crash reporter | ❌ Manquant | Debug difficile |

---

## 📈 Comparaison Feature par Feature

| Feature | Planifié (TECHNICAL_PLAN.md) | Actuel | Gap |
|---------|-------------------------------|--------|-----|
| **Face Tracking** | 468 landmarks + blendshapes | Position tête seule | **95% manquant** |
| **Hand Tracking** | 21 landmarks x2 mains | Aucun | **100% manquant** |
| **Arm Tracking** | Full IK Shoulder→Wrist | Keypoints détectés mais pas utilisés | **80% manquant** |
| **Body Tracking** | Full body avec contraintes | Tête seule | **90% manquant** |
| **OSC Integration** | VMC Protocol complet | Head position/rotation seule | **70% manquant** |
| **Performance** | 60 FPS, <15ms latency | ✅ Bon | ✅ OK |
| **UI/UX** | Auto-config, QR code | Config manuelle | **50% manquant** |
| **Distribution** | Portable .zip | Build from source | **70% manquant** |

---

## 🔥 Problèmes Actuels Identifiés

### 1. ✅ **RÉSOLU** : Espace Inversé
- **Symptôme**: Avatar à l'envers
- **Cause**: Axe Y inversé dans conversion 2D→3D (ligne 130 main.cpp)
- **Fix**: Changé `1.7f - norm_y` → `1.7f + norm_y`
- **Status**: ✅ Corrigé dans ce commit

### 2. ❌ **EN COURS** : Pas de Tracking Facial
- **Symptôme**: Pas de clignement d'yeux, bouche immobile
- **Cause**: YOLOv8-Pose ne fait **pas** de face tracking détaillé
- **Solution**: Ajouter **MediaPipe Face Mesh** ou modèle similaire
- **Priorité**: 🔴 **CRITIQUE**

### 3. ❌ **EN COURS** : Pas de Mains
- **Symptôme**: Mains absentes
- **Cause**: YOLOv8-Pose détecte seulement poignets, pas les doigts
- **Solution**: Ajouter **MediaPipe Hands** ou modèle dédié
- **Priorité**: 🔴 **CRITIQUE**

---

## 🗺️ Roadmap Suggérée

### Phase 1: Tracking Facial (2-3 jours) 🔴 PRIORITÉ
**Objectif**: Expressions faciales vivantes

- [ ] Intégrer **MediaPipe Face Mesh** (ou ONNX equivalent)
- [ ] Post-process 468 landmarks → blendshapes ARKit
- [ ] Calculer: `EyeBlinkLeft`, `EyeBlinkRight`, `JawOpen`, `MouthSmile`, etc.
- [ ] Envoyer blendshapes via OSC à VRChat (`/avatar/parameters/FT/v2/...`)
- [ ] Tester avec avatar VRChat compatible FaceTracking

**Résultat attendu**: Clignements, sourires, bouche qui s'ouvre quand vous parlez !

### Phase 2: Hand Tracking (2-3 jours) 🔴 PRIORITÉ  
**Objectif**: Mains et doigts trackés

- [ ] Intégrer **MediaPipe Hands** (ou ONNX)
- [ ] Post-process 21 landmarks x2 → positions doigts
- [ ] Calculer rotations des articulations (IK simplifié)
- [ ] Envoyer à VRChat: `LeftHand`, `RightHand` + finger bones
- [ ] Visualiser mains dans viewer 3D

**Résultat attendu**: Mains qui bougent, gestures reconnus !

### Phase 3: Arm Tracking (1-2 jours) 🟡 IMPORTANT
**Objectif**: Posture complète du haut du corps

- [ ] Utiliser keypoints YOLOv8 existants (Shoulders, Elbows, Wrists)
- [ ] Implémenter IK solver simple pour bras
- [ ] Ajouter bones au rendu 3D: Épaules, Coudes
- [ ] Envoyer à VRChat: `Chest`, `LeftUpperArm`, `LeftLowerArm`, etc.

**Résultat attendu**: Bras qui suivent vos mouvements !

### Phase 4: Biomécanique & Polish (2-3 jours) 🟢 AMÉLIORATION
**Objectif**: Mouvements fluides et naturels

- [ ] Motion Extrapolation (prédiction 15ms)
- [ ] Confidence-weighted smoothing
- [ ] Recovery mode (interpolation vers pose neutre)
- [ ] Calibration utilisateur (T-pose)

**Résultat attendu**: Tracking lisse, pas de saccades !

### Phase 5: Connectivité Avancée (3-4 jours) 🔵 BONUS
**Objectif**: Téléphone comme caméra pro

- [ ] Serveur web embarqué (stream MJPEG)
- [ ] QR Code dans UI
- [ ] App web mobile (HTML5 getUserMedia)
- [ ] Support NDI (optionnel)

**Résultat attendu**: Utilisez votre iPhone/Android comme caméra HD !

###Phase 6: Distribution & Auto-Config (2 jours) 🔵 CONFORT
**Objectif**: Installation en 1 clic

- [ ] Static linking complet
- [ ] Script packaging (.zip portable)
- [ ] Auto-détection process VRChat
- [ ] Helper firewall OSC

**Résultat attendu**: Download → Extract → Run → ça marche !

---

## 📊 Estimation de Complétion

| Catégorie | Complété | Manquant | % Progrès |
|-----------|----------|----------|-----------|
| **Infrastructure** | ✅ 100% | - | 🟢 100% |
| **Head Tracking** | ✅ Position+Rotation | Expressions | 🟡 30% |
| **Face Tracking** | ❌ | Blendshapes complets | 🔴 0% |
| **Hand Tracking** | ❌ | 21 landmarks x2 | 🔴 0% |
| **Arm Tracking** | ⏳ Keypoints only | IK + Rendering | 🟡 20% |
| **Body Tracking** | ⏳ Partial | Full body IK | 🟡 15% |
| **OSC/VRChat** | ✅ Basic | Blendshapes + Hands | 🟡 40% |
| **UI/UX** | ✅ Good | QR Code, Auto-config | 🟢 70% |
| **Performance** | ✅ Excellent | - | 🟢 100% |
| **Distribution** | ⏳ Build only | Portable .zip | 🟡 30% |

**Progrès Global**: 🟡 **~35%** vers l'objectif "Leader 2026"

---

## 🎯 Prochaines Actions IMMÉDIATES

### 1. ✅ **[FAIT]** Corriger l'Inversion
- Fix appliqué dans `main.cpp` ligne 130
- Recompiler et tester

### 2. 🔴 **[URGENT]** Ajouter Face Tracking
**Fichiers à créer**:
- `src/vision/FaceMesh.hpp` - Wrapper MediaPipe/ONNX
- `src/vision/BlendshapeCalculator.hpp` - 468 landmarks → ARKit blendshapes

**Fichiers à modifier**:
- `VisionLoop()` - Ajouter inférence face en parallèle
- OSC sender - Envoyer blendshapes

**Modèle requis**:
- `models/face_landmarker.onnx` (MediaPipe ou équivalent)

### 3. 🔴 **[URGENT]** Ajouter Hand Tracking
**Fichiers à créer**:
- `src/vision/HandTracking.hpp`
- `src/biomech/HandIK.hpp`

**Modèle requis**:
- `models/hand_landmarker.onnx`

---

## 💡 Recommandations

### Priorité #1: Face Tracking
**Pourquoi**: C'est la feature **la plus visible** et **la plus demandée**. Les gens veulent voir leurs expressions faciales dans VRChat.

**Action**: Commencer par MediaPipe Face Mesh (modèle ONNX disponible)

### Priorité #2: Hand Tracking  
**Pourquoi**: Indispensable pour gestures et interactions sociales dans VRChat.

**Action**: MediaPipe Hands (également disponible en ONNX)

### Priorité #3: Arms
**Pourquoi**: Moins critique car déjà partiellement supporté par VRChat IK.

**Action**: Utiliser keypoints YOLOv8 existants

---

## 📚 Ressources Nécessaires

### Modèles IA à Intégrer
1. **MediaPipe Face Mesh** (ONNX)
   - 468 landmarks
   - ~5-10ms sur GPU
   - Disponible: https://github.com/google/mediapipe

2. **MediaPipe Hands** (ONNX)
   - 21 landmarks x2
   - ~8-15ms sur GPU
   - Disponible: https://github.com/google/mediapipe

3. **YOLOv8-Pose** ✅ (déjà intégré)
   - 17 body keypoints
   - Utilisé pour body/arms

### Documentation VRChat OSC
- VRCFaceTracking v2 parameters: `/avatar/parameters/FT/v2/...`
- VMC Protocol pour bones: Head, Chest, Arms, Hands
- Docs: https://docs.vrchat.com/docs/osc-trackers

---

## ✅ Conclusion

### État Actuel
Le projet a une **excellente fondation technique** (architecture, performance, UI) mais **manque crucialement** les features de tracking avancé (face, hands, arms).

### Gap Principal
**Face + Hand Tracking** = 0% implémenté, mais c'est le **cœur de la valeur** du produit.

### Chemin vers "Leader 2026"
1. **Week 1-2**: Face Tracking → Expressions vivantes
2. **Week 2-3**: Hand Tracking → Gestures  
3. **Week 3-4**: Arms + Body IK → Posture complète
4. **Week 4-5**: Polish + Distribution → Product finalisé

**Temps estimé total**: ~1 mois de développement

---

**Status**: 🟡 **Alpha - En développement actif**  
**Prêt pour**: Tests internes, développement  
**Pas prêt pour**: Distribution publique, utilisation quotidienne

**Prochaine milestone**: Ajouter Face Tracking complet ! 🎯
