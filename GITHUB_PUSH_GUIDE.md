# Guide de Push vers GitHub

## 📦 Préparation (FAIT ✅)

- ✅ `.gitignore` mis à jour
- ✅ `.gitkeep` ajouté dans `models/` et `scripts/cloudflared/`
- ✅ Documentation à jour (README, TECHNICAL_PLAN, PROJECT_STATUS)

## 🚀 Commandes Git pour Push

### 1. Vérifier le statut

```bash
git status
```

### 2. Ajouter tous les fichiers

```bash
git add .
```

### 3. Commit avec message descriptif

```bash
git commit -m "Phase 6: GitHub Pages + Cloudflare Tunnel integration

- Created docs/ with modern glassmorphism interface
- Implemented CloudflareTunnel.hpp with auto-download
- Modified MainWindow.hpp for dynamic QR code
- Integrated tunnel auto-start in main.cpp
- Updated documentation (README, TECHNICAL_PLAN, PROJECT_STATUS)
- Fixed libqrencode dependency and HTTP POST server

Features:
- Phone Link now works worldwide without network config
- Automatic HTTPS tunnel via Cloudflare
- Zero configuration for end users
"
```

### 4. Push vers GitHub

```bash
git push origin main
```

## 🌐 Activer GitHub Pages (APRÈS le push)

1. Aller sur <https://github.com/shazamifius/VRchatFACE-HAND-ARM-tracking>
2. **Settings** (en haut)
3. **Pages** (menu de gauche)
4. **Source** :
   - Branch: `main`
   - Folder: `/docs`
5. **Save**
6. Attendre 1-2 minutes

**URL finale** : `https://shazamifius.github.io/VRchatFACE-HAND-ARM-tracking`

## 📊 Structure Finale sur GitHub

```
VRchatFACE-HAND-ARM-tracking/
├── .gitignore                 ✅ Push
├── README.md                  ✅ Push
├── TECHNICAL_PLAN.md          ✅ Push
├── PROJECT_STATUS.md          ✅ Push
├── CMakeLists.txt             ✅ Push
├── vcpkg.json                 ✅ Push
├── src/                       ✅ Push (tout le code C++)
├── docs/                      ✅ Push (site GitHub Pages)
│   ├── index.html
│   ├── style.css
│   ├── app.js
│   └── README.md
├── scripts/                   ✅ Push
│   ├── download_models.py
│   └── cloudflared/
│       └── .gitkeep
├── models/                    ✅ Push (vide avec .gitkeep)
│   └── .gitkeep
├── build/                     ❌ Ignoré (.gitignore)
└── vcpkg/                     ❌ Ignoré (.gitignore)
```

## ⚠️ Important

**Avant de cloner le projet (pour autres utilisateurs)** :

1. Installer les dépendances :

   ```bash
   python scripts/download_models.py
   ```

2. Compiler :

   ```bash
   cmake -B build -S . -DCMAKE_TOOLCHAIN_FILE=vcpkg/scripts/buildsystems/vcpkg.cmake
   cmake --build build --config Release
   ```

Le `cloudflared.exe` se téléchargera automatiquement au premier lancement !
