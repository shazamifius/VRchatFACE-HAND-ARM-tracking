# 🧠 Calibration System Design (VRCFT Standard)

## ❓ Pourquoi c'est INDISPENSABLE ?
Actuellement, votre analyse est **100% correcte** :
- **Problème** : Les valeurs (seuils) dans `BlendshapeCalculator` sont "en dur" (`0.28` pour EAR Open, `0.035` pour Smile).
- **Conséquence** :
    - Un utilisateur avec des "petits yeux" sera détecté comme "mi-clos" en permanence.
    - Un utilisateur avec des sourcils bas devra forcer comme un fou pour atteindre 1.0 en "Surprise".
    - Les "valeurs au pif" (hardcoded) ne marchent que pour un visage "moyen" théorique.

**La Calibration est l'étape qui transforme un "Gadget" en "Outil Pro".**

---

## 🛠 Le Concept Technique : "Zero-to-One" Mapping

Au lieu d'utiliser des constantes, nous allons capturer des **Intervalles Utilisateur**.

### Structure de Données
```cpp
struct UserCalibration {
    // Yeux
    float ear_open_min = 0.25f;  // Valeur min pour considérer l'œil ouvert (fatigue ?)
    float ear_open_max = 0.35f;  // Valeur max (yeux écarquillés)
    float ear_closed = 0.15f;    // Valeur réelle quand l'utilisateur ferme les yeux

    // Bouche
    float mouth_width_neutral;   // Largeur au repos
    float mouth_width_smile;     // Largeur max sourire (pour normaliser le smile)
    
    // Sourcils
    float brow_y_neutral;        // Hauteur sourcils au repos
    float brow_y_raised;         // Hauteur max surprise
    float brow_y_frowned;        // Hauteur min colère
};
```

---

## 📸 Workflow proposé (Wizard UI)

L'idée de demander "3 types de..." est bonne pour de l'IA (entrainement), mais pour de l'algorithmique directe (Blendshapes), nous avons surtout besoin des **EXTRÊMES**.

Voici le workflow optimisé (inspiré de iPhone/VRCFT) :

### Étape 1 : Le "NEUTRE" (Indispensable)
*   **Instruction** : "Regardez la caméra, visage détendu, sans expression."
*   **Capture** : 
    *   `Baseline Brows Y` (Hauteur sourcils repos)
    *   `Baseline Mouth Width` (Largeur bouche repos)
    *   `Baseline Eye Open` (Ouverture yeux standard)

### Étape 2 : Le "SMILE MAX"
*   **Instruction** : "Faites votre plus grand sourire !"
*   **Capture** : 
    *   `Max Mouth Corners Y` (Hauteur coins max) -> Définit le 1.0 du blendshape Smile.

### Étape 3 : Le "SURPRISE / BROWS UP"
*   **Instruction** : "Levez les sourcils au maximum (Choqué) !"
*   **Capture** :
    *   `Max Brow Y` -> Définit le 1.0 du blendshape BrowUp.

### Étape 4 : "YEUX FERMÉS"
*   **Instruction** : "Fermez les yeux normalement."
*   **Capture** :
    *   `Min EAR` -> Définit le 1.0 du blendshape Blink.

---

## 🚀 Impact sur le Code

Actuellement dans `BlendshapeCalculator.cpp` :
```cpp
// AVANT (Rigide)
constexpr float EAR_OPEN = 0.28f; 
bs.eyeBlinkLeft = MapRange(ear, 0.18f, EAR_OPEN);
```

```cpp
// APRÈS (Calibré)
// calibration.ear_max = Valeur capturée à l'étape "Neutre" ou "Yeux grands ouverts"
// calibration.ear_min = Valeur capturée à l'étape "Yeux Fermés"
bs.eyeBlinkLeft = MapRange(ear, calibration.ear_min, calibration.ear_max);
```

## ✅ Conclusion
**C'est OUI, c'est même CRITIQUE.**
Sans ça, l'utilisateur final passera son temps à dire "ça marche mal" juste parce qu'il a une morphologie différente de la vôtre.

Je suis prêt à implémenter :
1.  Une structure `UserCalibration`.
2.  Une fenêtre ImGui `Calibration` avec des boutons "Capturer Neutre", "Capturer Smile", etc.
3.  La sauvegarde de ces valeurs (dans un fichier `.json` ?).
