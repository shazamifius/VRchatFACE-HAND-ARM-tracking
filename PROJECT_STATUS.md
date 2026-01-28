# 📊 État du Projet - VRChat Universal Video Bridge

**Date**: 2026-01-28
**Version Actuelle**: 0.3.5 (Alpha - Analysis Report)
**Status Global**: 🟢 FONCTIONNEL (Core) / 🔴 BUGS CRITIQUES (Phone Link)

---

## 🧐 Analyse vs Plan Technique (@[TECHNICAL_PLAN.md](file:///c%3A/Users/Administrateur/Desktop/VRchatFACE-HAND-ARM-tracking/TECHNICAL_PLAN.md))

### ✅ Ce qui a été BIEN fait (Conforme ou Mieux)

1. **Architecture Core (Multithread Lock-Free)**
    * **Implémenté**: `VisionLoop` tourne dans un thread dédié (`src/main.cpp`).
    * **Architecture**: Utilisation de `std::lock_guard` et shared state atomique.
    * **Performance**: Le profilage est intégré (`inference_time_us`).

2. **Intégration modèles AI (Avance sur le planning)**
    * **Face Tracking**: `FaceMesh` est chargé et calcule les blendshapes (implanté dans `VisionLoop`). Le `PROJECT_STATUS.md` précédent indiquait "A intégrer", mais le code est **déjà présent et actif**.
    * **Hand Tracking**: `HandTracking` est chargé et traite les poignets détectés par YOLO.
    * **Body Tracking**: YOLOv8-pose pleinement intégré.

3. **UI & Visualization**
    * **ImGui Complet**: Controle complet, preview caméra, preview 3D OpenGL Avatar.
    * **Debug Mode**: Sliders pour tester les blendshapes manuellement.

### ⚠️ Ce qui est MOINS BIEN fait / Divergences

1. **Gestion de la Connexion Téléphone (Phone Link)**
    * **Plan**: "Quick Tunnel -> URL Publique -> QR Code Dynamique".
    * **Réalité**: L'implémentation est **cassée** à deux niveaux (voir Bugs). L'architecture est bonne sur le papier (Cloudflare Tunnel), mais l'exécution du code UI et Web est défaillante.

2. **Tests Unitaires**
    * 16 Tests existent, mais **2 échouent** (`SkeletonSolverTest`, `ModelSelectorTest`). La rigueur "Industrielle" visée n'est pas encore atteinte sur la fiabilité des tests.

3. **Robustesse Scripts**
    * `check_braces.py` contient des chemins en dur (`C:\Users\shaza...`), ce qui le rend inutilisable pour d'autres développeurs.

---

## 🐛 Analyse des Bugs (Objectif Documenter - Pas Corriger)

### 🔴 Bug 1 : QR Code affiche l'IP locale au lieu du Tunnel

**Symptôme** : Le QR code renvoie vers `http://192.168.1.3:8080/` même si le tunnel Cloudflare est actif.
**Analyse du Code** (`src/main.cpp` & `src/ui/MainWindow.hpp`) :

1. `VisionLoop` démarre le `WebServer` et obtient l'IP locale.
2. `main()` initialise `MainWindow` avec cette IP locale, ce qui génère le premier QR code.
3. Un thread séparé lance le Tunnel Cloudflare et obtient bien l'URL publique (vu dans les logs).
4. Ce thread met à jour `g_appState.tunnel_url`.
5. **ERREUR**: La boucle principale (UI Loop) **ne vérifie jamais** si `g_appState.tunnel_url` a changé pour appeler `main_window.UpdateQRCode()`. L'UI reste donc figée sur l'initialisation (IP locale).

### 🔴 Bug 2 : Impossible de connecter le téléphone (malgré lien valide)

**Symptôme** : Le site charge, mais le flux vidéo ne part pas ("Impossible de connexion").
**Analyse du Code** (`assets/web/index.html` & architecture Web) :

1. Le téléphone charge le site depuis `shazamifius.github.io` (via le lien loggué).
2. Le script JS tente d'envoyer la vidéo via `fetch('/video', { method: 'POST' ... })`.
3. **ERREUR**: `fetch('/video')` est relatif au domaine courant (`github.io`). GitHub Pages est statique et refuse le POST (404/405).
4. **Correctif requis (Analyse)** : Le JS doit lire le paramètre `?tunnel=...` dans l'URL et construire l'adresse cible absolue : `https://<tunnel-url>/video`.

---

## 🧪 Rapport de Tests

| Commande | Résultat | Analyse |
| :--- | :--- | :--- |
| `unit_tests.exe` | ❌ FAILED | 14/16 Passés. Échecs: `SkeletonSolverTest.BasicSolve` (Confiance 0 vs 0.9) et `ModelSelectorTest.InitialState` (Mode mismatch). |
| `check_braces.py` | ❌ ERROR | Crash (Chemin absolu utilisateur hardcodé). |
| `Vision System` | ✅ OK | Logs confirment le chargement des 3 modèles (Pose, Face, Hand). |
| `Cloudflare` | ✅ OK | Tunnel créé avec succès (`hunter-bias-centered-knowledge...`). |

---

## 📝 Recommandations (Prochaines étapes pour le User)

1. **Priorité 1 (Phone Link)** : Modifier `assets/web/index.html` pour utiliser le paramètre `tunnel` pour les requêtes fetch.
2. **Priorité 2 (UI)** : Ajouter dans la boucle `main()` un check : `if (g_appState.tunnel_ready) { main_window.UpdateQRCode(g_appState.tunnel_url); ... }`.
3. **Priorité 3 (Qualité)** : Corriger les tests unitaires et rendre les scripts python portables.
