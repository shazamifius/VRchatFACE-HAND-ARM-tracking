# Plan d'Implémentation Technique - VRChat Universal Video Bridge (Leader 2026)

## 🏗️ Architecture "Data-Oriented" (Multithread Lock-Free)

### Objectif
Viser la place de **Leader du Marché** en éliminant les goulots d'étranglement CPU grâce à une architecture parallèle massive, optimisée au niveau cache, et une UX "Magique".

### 🚀 Le Volet "Installation & UX" (Le chaînon manquant)

#### 1. Zéro Dépendance Externe (Portable App)
- **Static Linking** : Compilation statique (`/MT`). Distribution en `.zip` portable (EXE + DLLs) sans installateur MSI.
- **VCPKG Manifest** : Gestion stricte des versions via manifest pour reproductibilité.
- **Redistribuables** : Inclusion ou lien statique de `vc_redist.x64.exe`.

#### 2. Auto-Configuration VRChat (Magie OSC)
- **Auto-Detection** : Détection du process VRChat.
- **Port Mapping** : Gestion intelligente du port `9000` (alerte ou auto-switch).
- **VRCFT Standard** : Envoi direct au format `/avatar/parameters/FT/v2/...` (Zero Config).

#### 3. Connexion Téléphone (Le mode "Setup de poche")
- **QR Code** : Affichage QR Code dans l'UI pour appairage rapide.
- **App Compagnon Web** : Flux vidéo direct via réseau local (remplace NDI complexe).

### 📐 Schéma du Pipeline de Données (Vue Utilisateur)
1. **Source** : Webcam (Auto-détectée) OU Téléphone (Via QR Code/NDI).
2. **Processing** : Moteur "Zero-Overhead" (Silencieux, en tâche de fond).
3. **Mapping** : Traduction automatique vers `VRCFaceTracking` + `VMC`.
4. **VRChat** : L'avatar bouge instantanément sans avoir ouvert un menu.

### 1. Le Pipeline Asynchrone (Le Moteur)
Architecture **Data-Oriented Design (DOD)** pour garantir 60fps constants même sur iGPU.

-   **Memory & Cache Optimization** :
    -   **False Sharing Protection** : Utilisation stricte de strucures **SoA (Structure of Arrays)** et alignement mémoire `alignas(64)` ou `std::hardware_destructive_interference_size` pour éviter la contention de cache L1/L2 entre threads.
    -   **Gestion Mémoire** : Utilisation de `std::pmr` (Polymorphic Allocators) pour les Ring Buffers et allocations temporaires (zéro allocation heap durant la boucle de tracking).
-   **Orchestration** : `Ring Buffer` Lock-free (SPSC queues) pour passer les données.
-   **Threads Dédiés** :
    -   **Thread 1 (Input)** : Capture Vidéo (OpenCV / NDI) → *Frame Buffer*.
    -   **Thread 2 (Preproc)** : Resize, Normalize, Upload GPU.
    -   **Thread 3 (Vision AI)** : Inférence ONNX Runtime (DirectML). Stratégie "Frame Dropping" si saturation.
    -   **Thread 4 (Biomécanique)** : Smoothing, IK Solver, Motion Extrapolation.
    -   **Thread 5 (Output)** : Sérialisation VMC/OSC + Network Dispatch.

### 2. Intelligence Artificielle "State-of-the-Art"
-   **Moteur** : **ONNX Runtime + DirectML** + **Custom Ops** (pour post-processing optimisé).
-   **Modèles** : Hybrid CNN+Transformer (YOLO-Pose / RTMPose), Distillation pour iGPU.
-   **Sélecteur Auto (Heuristique Avancée)** :
    -   **Phase 1 (Warm-up)** : Ignorer 3 premières frames (GPU warm-up).
    -   **Phase 2 (Benchmark)** : Mesure latence d'inférence sur 10 frames.
    -   **Phase 3 (Décision)** :
        -   < 12ms : Mode **Ultra**.
        -   12-18ms : Mode **Balanced**.
        -   > 18ms : Mode **Eco** (Résolution réduite / Modèle Nano).
    -   **Phase 4 (Adaptation)** : Monitoring continu pour downgrade/upgrade dynamique si spikes.

### 3. Couche Biomécanique "Avatar-Aware"
*Le secret du "Grade Industriel" : Ne jamais livrer de la "Raw Data".*

-   **Géométrie & Rotations** :
    -   **Full Quaternions** : Interdiction stricte des Angles d'Euler (éviter Gimbal Lock). Convention unique (x, y, z, w).
    -   **Coordinate System Conversion** : Conversion centralisée et rigoureuse `AI (Main Droite, Z-up)` → `Unity/VRChat (Main Gauche, Y-up)`. Pattern : `ConvertToUnity(const Quaternion& q_ai)`.
-   **Gestion de la Latence Perçue** :
    -   **Motion Extrapolation** : Prédiction temporelle (5-15ms) basée sur Pos + Vitesse pour compenser la latence réseau/OSC.
    -   **Timestamp & Clock Sync** : Timestamping de chaque frame pour compenser les délais variables (capture -> inférence -> réseau).
    -   **Confidence-Weighted Filtering** : Filtrage adaptatif (Joint incertain = fort lissage ; Joint stable = haute réactivité).
-   **Recovery Mode** : Interpolation intelligente vers pose neutre en cas de perte de tracking (pas de "snap" brutal).

### 4. UX & "Magic Features"
-   **Auto-Hardware Profile** : Détection auto et configuration optimale (zéro clic).
-   **Feedback Visuel** : Heatmap squelette temps-réel (confiance/occlusions).
-   **Calibration 1-Clic** : Mesure automatique de l'échelle utilisateur.

### 5. Infrastructure & Sécurité
-   **CI/CD** : **GitHub Actions** pour compilation automatique Release et checks de dépendances à chaque push.

## 📅 Plan d'Action "Pro"

### Phase 0: La Fondation (Architecture & Threads)
1.  Setup CMake + vcpkg + **GitHub Actions**.
2.  Implémentation **Ring Buffers** (`std::pmr`, `alignas(64)`).
3.  Création **Profiler Interne** (Mesure µs).

### Phase 1: Pipeline Vision
1.  Intégration ONNX Runtime + DirectML.
2.  **Sélecteur Qualité** (Benchmark démarrage).

### Phase 2: Biomécanique & Post-Processing
1.  Implémentation **Maths Library** (**GLM** ou Eigen) pour Quaternions.
2.  **Coordinate Converter** (AI -> Unity).
3.  **Motion Extrapolation** & Timestamping.
4.  Solver IK + Contraintes.

### Phase 3: Connectivité
1.  VMC Protocol (OSC).
2.  Support NDI.

### Phase 4: Polish & UX
1.  UI ImGui "Gamer" + **QR Code View**.
2.  **Auto-Configuration VRChat** (OSC/Port).
3.  Presets & Calibration.

### Phase 5 : "Seamless Experience" & Distribution
1.  **Bundling** : Modèles IA dans dossier `models/`.
2.  **Auto-Updater** : Vérification updates GitHub.
3.  **Firewall Helper** : Whitelist ports OSC/NDI.
4.  **Crash Reporter** : Mini-dump system.

## 📊 Stack Technique Validée
| Composant | Technologie |
| :--- | :--- |
| **Langage** | **C++20** (Concepts, std::pmr, hardware_destructive_interference_size) |
| **Maths** | **GLM** (optimisé Game Dev / OpenGL) |
| **Concurrence** | **Lock-free Queues** (MoodyCamel) + **SoA Design** |
| **Inférence** | **ONNX Runtime** + **DirectML** + **Custom Ops** |
| **Input** | **OpenCV** + **NDI SDK** |
| **Output** | **VMC Protocol** (OSC) |
| **UI** | **Dear ImGui** |
| **CI/CD** | **GitHub Actions** |
