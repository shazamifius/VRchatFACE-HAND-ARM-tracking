# Avancement du Projet - État au 19/02/2026

Ce document recense l'intégralité des fonctionnalités, l'architecture technique et les dernières mises à jour du projet **VRChat Bridge Hub**.

## 🎯 Objectif

Offrir un système de tracking facial et manuel (Face + Hand + Arm) de haute précision pour VRChat, sans équipement coûteux (Webcam ou Téléphone), avec une performance native.

---

## 🏗️ Architecture Technique (Rust Native)

L'application est entièrement construite en **Rust** pour garantir une latence minimale et une robustesse maximale.

* **Backend**: Tauri + Rust (Logique de tracking, Inférence AI, Réseau OSC).
* **Frontend**: HTML/JS (Interface utilisateur, visualisation caméra).
* **AI Engine**: `ort` (ONNX Runtime) exécutant des modèles MediaPipe optimisés sur CPU/GPU.

### Performance

* **FPS Cible**: 30 FPS stable (récemment validé à 19-30 FPS selon l'éclairage).
* **Latence**: < 15ms de traitement (Inférence + Solver).

---

## 🛠️ Audit du Système IA & Refonte Globale (27/02/2026)

Après une exploration complète de l'architecture, il s'avère que le projet repose sur d'excellentes bases (utilisation de MediaPipe via `ort`, algorithmes PnP et filtrage OneEuro) mais que l'IA **souffre de défauts d'incohérences de coordonnées bloquant intégralement le tracking réel**.

### Les Incohérences Dénoncées (Pourquoi l'IA "ne marche pas")

1. **Le "Bug de l'Espace" (Explosion des coordonnées)** : L'IA (`ai.rs` / `landmarks.rs`) sort des points de repères en **pixels natifs de l'image** (ex: 320x240). Cependant, le module de conversion OSC (`solver.rs`) croit que les points sont au format normalisé 0..1 et les remultiplie par la résolution (640x480). Résultat : Le solveur PnP traite des visages larges de milliers de kilomètres, ce qui plante les rotations et positions de la tête en sortant des erreurs infinies (`NaN`).
2. **Le Faux Suivi Oculaire et Auto-Clignements excessifs** : Pour pallier au manque de tracking réel (dû au bug 1), `solver.rs` a été configuré pour injecter des mouvements *synthétiques* importants ("Alive Feel", Saccades, Auto-Blink). L'IA est court-circuitée par ces animations procédurales, qui bloquent ou interfèrent fortement avec un clignement réel d'un utilisateur.
3. **Pertes de la dimension Z (Profondeur)** : Lors de la conversion des prédictions (modèles 3D Face et Hand), le recadrage local n'applique pas l'échelle de profondeur Z convenablement par rapport à la boîte de capture initiale de MediaPipe.
4. **Calcul de la main arbitraire (Handedness)** : Identifier quelle est la droite ou la gauche de la main est géré empiriquement, sans bien compenser l'effet "miroir" de la webcam.

### Fichiers Importants à CONSERVER (Ne PAS supprimer !)

Le backend existant contient des pépites qu'il faut absolument conserver pour la performance :

* `models/*.onnx` : Ne pas modifier, ils demandent juste à ce qu'on interprète correctement leurs valeurs en sortie.
* `hub/src-tauri/src/tracking/blaze/` (`config.rs`, `detector.rs`, `utils.rs`) : Très bonne implémentation manuelle du post-processing ONNX. Le pipeline de détection est rapide !
* `hub/src-tauri/src/tracking/filter.rs` et `ik.rs` : Le filtre "OneEuro" et la cinématique inverse (IK) calculant les coudes sont complexes et de très bons modules mathématiques pour éviter la tremblote (jitter).
* `hub/src/main.js` & Interface Vidéo HTML : Le code frontend de visualisation et d'interface Tauri est très robuste. L'affichage s'opérera tout seul dès que les données IA d'arrière-plan cesseront d'être corrompues par le bug d'échelle.

---

## 🛠️ Fonctionnalités Implémentées (Réelles)

### 1. Face Tracking (En cours de sauvetage)

* **Pipeline Modèle**: BlazeFace + Mesh 468 points fonctionnel mais ses données subissent l'écrasement des faux signaux (Alive feel) et l'explosion de données (scale pixels * scale d'écran).

### 2. Hand & Arm Tracking

* **État**: Détection paume + Landmarks (21 points) ok, mais le Z-axis doit s'ajuster correctement dans les calculs locaux de la main pour rendre le solveur coude (`ik.rs`) parfaitement précis.

### 3. Gestion Caméra & Système

* **Smart Retry Logic**: Le système contourne bien la caméra bloquée en essayant de forcer 15/30fps ou des changements de résolutions. Fonctionne.
* **Téléphone MJPEG App**: Intégré.

---

## 📅 Roadmap Immédiate (Refonte de l'IA)

1. **Refonte de l'Interfaçage des Pipelines `ai.rs` et `solver.rs`**:
   * Passer toutes les données de sortie IA en espace normalisé absolu `[0.0 ; 1.0]`.
   * Permettre à `solver.rs` de faire la multiplication `* resolution` proprement et à l'affichage frontend `main.js` de scaler par dessus le `<canvas>`.
2. **Réduire l'Alive Feel**: Le temps de stabiliser l'extraction oculaire et l'ouverture de bouche (JawOpen), désactiver les saccades et auto-clignements forcés qui nuisent au vrai tracking de l'utilisateur.
3. **Tester sur Webcam** via relancement du build UI.
