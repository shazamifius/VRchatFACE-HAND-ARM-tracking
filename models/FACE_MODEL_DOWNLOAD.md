# Face Mesh Model Download Instructions

## 📦 Modèle Requis

Pour le Face Tracking, nous avons besoin d'un modèle **MediaPipe Face Mesh** au format ONNX.

---

## Option 1: MediaPipe Face Mesh (Recommandé)

### Source
- **Hugging Face**: `py-feat/mp_facemesh_v2`
- **URL**: https://huggingface.co/py-feat/mp_facemesh_v2

### Téléchargement

1. Visitez: https://huggingface.co/py-feat/mp_facemesh_v2/tree/main
2. Cherchez un fichier `.onnx` dans l'onglet "Files and versions"
3. Téléchargez le fichier
4. Placez-le dans: `models/face_landmarker.onnx`

### Alternative: Conversion depuis PyTorch

Si seul le `.pth` est disponible :
```bash
pip install onnx torch
python scripts/convert_facemesh_to_onnx.py
```

---

## Option 2: Lightweight Alternative

### Face Detection + Landmarks (68 points)

**Dlib-based model** (plus léger mais moins précis):
- **URL**: https://github.com/onnx/models/tree/main/vision/body_analysis/age_gender
- **Fichier**: `facial-landmarks-68.onnx`
- **Taille**: ~10MB
- **Landmarks**: 68 (vs 468 MediaPipe)

**⚠️ Limitation**: Moins de détails pour expressions subtiles

---

## Option 3: Solution Temporaire - Stub Mode

Pour tester l'intégration sans modèle :
- Le code utilise actuellement un **mock/stub** qui retourne des landmarks neutres
- Permet de valider l'architecture avant d'avoir le vrai modèle
- À remplacer dès que le modèle ONNX est disponible

---

## 🎯 Prochaine Action

**ÉTAPE ACTUELLE**: Le code est écrit avec un stub. Dès que vous obtenez un modèle ONNX :

1. Placez-le dans `models/face_landmarker.onnx`
2. Le système le chargera automatiquement au démarrage
3. Le Face Tracking sera activé !

---

## 📝 Format Attendu du Modèle

### Input
- **Shape**: `[1, 3, 256, 256]` ou `[1, 3, 192, 192]`
- **Type**: `float32`
- **Range**: `[0, 1]` (normalisé)
- **Format**: RGB

### Output
- **Shape**: `[1, 468, 3]` pour MediaPipe
- **Type**: `float32`
- **Format**: `[x, y, z]` normalisé `[0, 1]`

---

## ✅ Status

- [ ] Modèle téléchargé
- [x] Code d'intégration écrit
- [x] Stub/Mock fonctionnel pour tests
- [ ] Modèle actif et fonctionnel

**Current Mode**: 🟡 STUB (en attente du modèle)
