# 📐 Tracking Space Contract

Ce document définit officiellement les conventions d'espace tridimensionnel (3D) utilisées dans l'ensemble du pipeline **VRChat Bridge Hub**, depuis la sortie brute de l'IA (MediaPipe) jusqu'à la transmission finale vers le client ou le driver XR (Option B : OpenVR / OpenXR).

Il est impératif que toutes les transformations, filtres, et solveurs mathématiques respectent ce contrat pour garantir la stabilité du tracking et éviter toute distorsion liée aux résolutions de caméras.

---

## 1. Pipeline Interne : Espace Normalisé Universel

Afin d'être totalement agnostique vis-à-vis des sources vidéo (Webcam 16:9, Téléphone 4:3 ou 1:1, etc.), les données transitant *entre* le module IA (`ai.rs`) et le module de résolution (`solver.rs`) seront strictement en **Normalized Space** `[0.0 ; 1.0]`.

* **X (Horizontal)** : `[0.0, 1.0]` où `0.0` = Gauche de l'image, `1.0` = Droite de l'image.
* **Y (Vertical)** : `[0.0, 1.0]` où `0.0` = Haut de l'image, `1.0` = Bas de l'image.
* **Z (Profondeur)** : Il est **relatif au X-Scale** de l'image. Il n'est **pas** en pixels absolus, et sa valeur est proportionnelle à la taille de la boîte englobante d'origine.
  * *Règle d'or Z :* Ne jamais additionner ou mélanger la profondeur Z de base avec des pixels absolus avant que le tout ne soit projeté dans l'espace canonique.

### Justification

* Indépendant des résolutions (480p ou 1080p ont le même `[0,1]`).
* Indépendant de la source physique (Téléphone, Webcam).
* Mathématiquement plus robuste pour prévenir les débordements (NaN/Infinity) dans le solveur.

---

## 2. Squelette Canonique : Espace Métrique & Solveur (Solver.rs)

Pour que la cinématique inverse (IK) pour les bras et les résolutions PnP (Perspective-n-Point) soient réalistes, nous projetons cet espace normalisé dans un **Espace Physique Canonique** (en mètres) uniquement à l'intérieur de `solver.rs`, pile au moment du calcul.

* **Unité interne** : **MÈTRE** (`1.0 = 1mètre`).
* **Orientation des Axes (Repère Droitier - Right-Handed)** :
  * **+X** : Vers la Droite du monde.
  * **+Y** : Vers le Haut (Sky).
  * **+Z** : Recule vers Toi (Negative Z : -Z s'enfonce dans le monde/vers la webcam).
* **Origine `(0, 0, 0)`** :
  * Pour le solveur tête : Centre logique de la caméra avec profondeur déduite.
  * Pour la racine du joueur (Avatar Root) : Position spatiale de la Tête / HMD projetée au sol selon la calibration.

---

## 3. Architecture Mathématique de la Transformation XR (Option B)

L'objectif de transformer l'application en Vrai Driver XR (OpenXR / SteamVR) exige que le squelette canonique soit dérivé sans heurts en devices de tracking 6DoF (Position + Rotation) simulés.

### A. Flux de données

1. **Pose HMD (Réelle)**
   * Obtenue nativement depuis la runtime XR.
   * Sert d'origine absolue pour nos "pseudo-trackers" afin que la vue caméra corresponde aux mouvements de l'utilisateur dans l'espace virtuel.
2. **Traduction Offset (Épaules)**
   * On calcule un point d'épaule statique (ou semi-dynamique) basé sur `HMD POS + Offset Calibré` (`Y = HMD_Y - 0.25m`).
3. **Résolution des Poignets (Controllers / Hands 6DoF)**
   * Par rapport à la projection de notre caméra webcam normalisée, on situe la direction du poignet (IK Coude-Poignet).
   * On assigne la position finale du solveur comme Position (Vector3) du faux contrôleur.
   * On déduit la rotation (Quaternion) à partir du centre de la paume (calcul du vecteur normal du dos de la main entre les métacarpiens V, et II par rapport à I).
4. **Bone Local Transforms (Ext. Hand Tracking openXR)**
   * Si implémentée, la spec d'OpenXR ne demande plus un Controller 6DoF basique, mais des Bones Locaux.
   * Chaque point du réseau MediaPipe doit être transposé en Espace Local (la base de son parent, e.g. Poignet -> Metacarpe -> Phalange Proxy -> Phalange Distale).

---

## 4. Asserts Runtime & Rigueur Interne

Pour assurer ces fondations, le pipeline code appliquera les règles suivantes (via `debug_assert!` ou validation locale) :

1. **Assert Normalisé (`ai.rs -> solver.rs`)**
   * Tout point de Landmark (X, Y) entrant dans le solver a une assertion stricte confirmant sa plage théorique.
   * *Avertissement et soft clamp si des repères sortent de +0.1/-0.1 au-delà des bords liés au crop.*
2. **Déduction Gauche/Droite (Handedness)**
   * Strictement décorrélé de l'axe `X < 0.5`. Utiliser la *probabilité mathématique de la classification* (le `Score`/Flag que le modèle ONNX ressort pour Right/Left Hand). L'image inversée logicielle de la webcam est gérée via un simple flip mathématique au rendu sans altérer la donnée d'entrée du modèle.
3. **Protection de la "Vérité IA" (Alive Feel Isolés)**
   * L'Alive Feel (clignements synthétiques, mouvements oculaires forcés) est purement Post-Solve.
   * Si `Face-Tracking Confidence > 80%` (et absence de perte) -> le Bypass est de `0%` (Les Fake saccades sont coupées net). Pas de mélange des sources de vérité.
   * Le Fake Alive tracking agit comme protection anti-gel, pas pour remplacer une mesure propre.
