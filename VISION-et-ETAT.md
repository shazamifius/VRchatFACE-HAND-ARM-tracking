# 🎭 VRChat Bridge Hub — l'idée, l'objectif, et où on en est

> *Transforme une simple webcam (ou ton téléphone) en système de capture de mouvement pour VRChat.*
>
> Un document simple pour expliquer ce qu'est le projet, ce qu'il veut devenir, et le chemin déjà parcouru.
> Écrit en regardant le code réel, les docs techniques, la carte du projet (graphify) et l'historique des commits — pas une plaquette commerciale, un état des lieux honnête.

---

## 1. En une phrase

**VRChat Bridge Hub, c'est un outil tout-en-un et open source qui anime ton avatar VRChat — visage, mains, bras, tête, corps — à partir d'une simple caméra, sans matériel coûteux.**

Pas de casque à 500 €, pas de trackers Vive, pas de stations de base. Une webcam ou ton téléphone suffisent. Tout tourne **en local sur ton PC** (ta vidéo ne part pas sur un serveur), en **Rust** pour la rapidité.

---

## 2. Le problème de départ

Dans la VR sociale (VRChat), ce qui rend un avatar *vivant*, ce n'est pas son apparence — c'est qu'il **reproduise tes expressions** : ton sourire, ton clin d'œil, tes gestes de mains, ta posture.

Le souci : historiquement, ça demande du **matériel cher**. Suivi facial, suivi du corps (Full Body Tracking), casque haut de gamme… On parle de centaines d'euros de capteurs infrarouges et de trackers. Une grosse barrière à l'entrée.

Or, depuis quelques années, l'IA de vision par ordinateur (les modèles **MediaPipe** de Google, par exemple) sait analyser un visage et des mains **à partir d'une simple image de webcam**, en temps réel, sur un PC normal. Le matériel cher devient remplaçable par du **logiciel malin**.

**C'est le pari du projet : démocratiser la « présence virtuelle » de qualité — la rendre accessible à tous, pas seulement à ceux qui peuvent payer le matériel.**

---

## 3. L'objectif : le « tout-en-un par webcam »

Le but n'est pas de faire *une* fonction de tracking, mais **toutes**, dans un seul outil simple :

- 🙂 **Visage** — sourire, ouverture de bouche, clignements, sourcils (maillage dense de 468 points).
- ✋ **Mains** — 21 points par main, chaque doigt, pour les gestes.
- 💪 **Bras** — la position du coude et de l'épaule, déduite de la main.
- 🙆 **Tête & corps** — orientation de la tête, posture du haut du corps.

Le tout avec une promesse d'**expérience « magique »** (l'ambition affichée dans les docs) :

| Principe | Ce que ça veut dire |
|---|---|
| 🚀 **Performance native** | Cœur en **Rust** — rapide, léger en CPU, faible latence |
| 📱 **Caméra du téléphone** | Scanne un **QR code**, ton téléphone filme et envoie l'image — zéro app à installer |
| 🔒 **Vie privée d'abord** | Tout tourne **en local**. Ta vidéo ne quitte pas ton PC (sauf si *tu* actives le mode distant) |
| ⚙️ **Zéro config** | Détection automatique de VRChat, branchement automatique sur le bon « tuyau » (OSC) |
| 🌍 **Marche partout** | Mode **Cloudflare Tunnel** optionnel : téléphone en 4G et PC en Wi-Fi, ça marche quand même |

---

## 4. Comment ça marche, expliqué simplement

Le voyage d'un mouvement, de ta caméra jusqu'à ton avatar :

```
   Caméra (webcam OU téléphone via QR code)
            │
   ① L'IA analyse l'image (modèles MediaPipe via ONNX)
      → des "points de repère" : 468 sur le visage, 21 par main…
            │
   ② Le SOLVEUR transforme ces points en mouvements réels
      → "la mâchoire est ouverte à 0.7", "le coude est ici" (IK), 
        "la tête est tournée comme ça" (PnP)
            │
   ③ Un FILTRE lisse tout (anti-tremblote — filtre "One Euro")
            │
   ④ Envoi vers VRChat par DEUX chemins possibles :
        • OSC  → le langage natif que VRChat écoute (port 9000)
        • Driver SteamVR → un FAUX casque + fausses manettes
                            que le PC croit réels ("Option B")
```

### Les mots techniques, vulgarisés

- **Landmarks (points de repère)** : l'IA ne « comprend » pas un visage, elle pose des points dessus (coin de l'œil, commissure des lèvres…). 468 pour le visage, 21 par main, 33 pour le corps.
- **OSC** : la « langue » que VRChat écoute sur un port réseau. On lui dit `JawOpen = 0.7` et l'avatar ouvre la bouche.
- **IK (cinématique inverse)** : deviner où est le coude quand on connaît la main et l'épaule. C'est ce qui fait que le bras a l'air naturel.
- **PnP** : deviner l'orientation 3D de la tête à partir de quelques points 2D de l'image.
- **Driver SteamVR** : au lieu de parler à VRChat par OSC, on crée un **faux casque et de fausses manettes**. Le PC croit qu'un vrai équipement VR est branché — ce qui débloque le suivi du corps « comme un vrai ».

### La règle d'or du projet (le « contrat d'espace »)

> **Entre l'IA et le solveur, toutes les coordonnées sont normalisées entre `0.0` et `1.0`.**

Pourquoi c'est crucial : peu importe que la caméra fasse du 480p ou du 1080p, qu'elle soit un téléphone 4:3 ou une webcam 16:9 — un point au milieu de l'image vaut toujours `0.5`. Ça rend les maths **robustes** et empêche les explosions de valeurs (voir le bug majeur en partie 6). C'est écrit noir sur blanc dans `TRACKING_SPACE_CONTRACT.md`.

---

## 5. L'architecture : les morceaux du projet

Le projet est en plusieurs blocs qui se complètent :

| Dossier | Ce que c'est | Rôle |
|---|---|---|
| **`hub/`** | L'application principale — **Rust + Tauri**, interface « glass » sombre | Le cerveau : caméra, IA, solveur, réseau, UI |
| **`hub/.../tracking/`** | Le cœur du tracking (≈ 20 modules Rust) | `ai`, `solver`, `ik`, `pnp`, `filter`, `camera`, `calibration`, `vmt`… |
| **`hub/.../models/`** | Les 6 modèles d'IA (ONNX) | Visage, mains, paume, corps — fournis avec |
| **`driver/`** | Le **driver SteamVR** (C++) | Le « faux casque/manettes » (Option B) |
| **`diag/`** | Une sonde de diagnostic OpenVR (C++) | Outil pour déboguer ce que SteamVR voit vraiment |
| **`assets/web/`** | La page web du téléphone | Ce que ton téléphone ouvre après le QR code |

Une **boîte à outils de scripts** (`START_PROJECT.bat`, `DIAGNOSE.ps1`, `osc_debugger`…) entoure le tout pour lancer, construire et déboguer facilement.

---

## 6. Où on en est vraiment (l'état honnête)

C'est ici qu'il faut être franc, parce que la vitrine et la réalité du chantier ne disent pas tout à fait la même chose — et c'est **normal pour un projet ambitieux en cours**.

> Le `README.md` affiche fièrement « Status: Stable ». La réalité, racontée par le propre journal du projet (`avancer_du_projet.md`) et par les commits récents, est celle d'un projet aux **fondations excellentes mais au tracking encore en cours de fiabilisation**. Ce n'est pas un défaut — c'est là qu'en est le travail.

### ✅ Ce qui est solide (les « pépites » à garder)

D'après l'audit interne du projet lui-même :

- La **détection** (pipeline BlazeFace + post-traitement ONNX manuel) est rapide et bien faite.
- Le **filtre anti-tremblote** (One Euro) et la **cinématique inverse du coude** (IK) sont de très bons modules mathématiques.
- La **gestion caméra** (retente automatiquement en cas de caméra bloquée, force 15/30 fps) fonctionne.
- Le **flux téléphone** (le téléphone envoie ses images au PC) est intégré, avec tunnel Cloudflare pour le mode distant.
- L'**interface Tauri** et la visualisation vidéo sont robustes.
- Tout le pan **driver SteamVR + sonde de diagnostic** existe et est activement débogué.

### 🟡 Ce qui était cassé et en cours de sauvetage

Le projet a identifié, honnêtement, **pourquoi « l'IA ne marchait pas »** :

1. **Le « bug de l'espace » (le gros)** : l'IA sortait des points en **pixels** (ex. 320×240), mais le solveur croyait qu'ils étaient déjà normalisés (`0..1`) et les re-multipliait par la résolution. Résultat : des visages « larges de milliers de kilomètres », des calculs qui explosent en erreurs infinies (`NaN`), et le tracking qui plante.
   → **La parade existe** : le contrat d'espace normalisé (partie 4) a été écrit précisément pour corriger ça.
2. **Le « faux vivant » (Alive Feel)** : pour masquer le bug n°1, on avait ajouté des clignements et mouvements d'yeux **synthétiques**… qui court-circuitent le vrai suivi. La feuille de route prévoit de les **réduire** pour laisser passer le mouvement réel de l'utilisateur.
3. **La profondeur (axe Z)** et **la main gauche/droite** : encore à fiabiliser.

### 🔄 Le virage récent : du « OSC » vers le « vrai driver VR »

Les derniers commits racontent une histoire claire : le projet est en train de **basculer vers l'intégration SteamVR** (le faux casque/manettes) et débogue des problèmes **bien réels en jeu** :

- *« Espacer les manettes aux proportions humaines »* (corrige un avatar minuscule de ~10 cm).
- *« Ancrer les manettes sur l'orientation de la tête »* (les mains ne tournent plus autour de la tête).
- *« Orientation de la tête = souris uniquement »* — la pose de tête par webcam était **trop instable**, donc temporairement remplacée par la souris. Décision honnête et pragmatique.

### ⬜ Ce qui reste devant (la feuille de route immédiate)

1. **Finir la refonte `ai.rs` ↔ `solver.rs`** en espace normalisé (appliquer le contrat partout).
2. **Calmer l'« Alive Feel »** pour rendre la main au vrai tracking (yeux + ouverture de bouche).
3. **Re-tester sur webcam** une fois le pipeline assaini.
4. Plus loin (vision des docs techniques) : sélecteur de qualité automatique selon le matériel, extrapolation de mouvement pour masquer la latence, distribution « portable » zéro-installation, mises à jour automatiques.

---

## 7. L'ambition de fond

Au-delà du code, deux documents (`Outil VRChat Open Source Tout-en-un.md` et `TECHNICAL_PLAN.md`) posent le **cap** :

- Devenir **la référence open source** du tracking « tout-en-un par webcam » pour la VR sociale.
- Une expérience **« sans friction »** : binaire portable, auto-configuration, calibration en un clic, QR code pour le téléphone.
- Une infrastructure **« Phone Link » mondiale** (page web GitHub Pages + tunnel Cloudflare) pour que n'importe qui, n'importe où, utilise son téléphone comme caméra **sans rien configurer**.

C'est un cap exigeant — et le code actuel en a déjà construit une bonne partie. L'écart entre l'ambition « grade industriel » et l'état « en fiabilisation » est exactement ce sur quoi porte le travail en ce moment.

---

## 8. Crédits

Le projet s'appuie, honnêtement, sur de beaux travaux open source :

- **AlbertaBeef** (`blaze_app_python`) — l'analyse des modèles MediaPipe qui a inspiré la détection.
- **Google MediaPipe** — les modèles BlazeFace / BlazeLandmark.
- **Rust**, **Tauri**, **ONNX Runtime**, **Cloudflare**, et les crates `tokio` / `axum` (serveur web du téléphone), `nalgebra` (maths 3D), `ros_osc` (dialogue avec VRChat).

---

## 9. En résumé

- **Le projet aujourd'hui** = un outil tout-en-un de tracking VRChat (visage + mains + bras + tête + corps) par webcam/téléphone, en Rust, 100 % local. **Fondations solides, tracking en cours de fiabilisation.**
- **Le grand obstacle** (le « bug de l'espace » des coordonnées) est **diagnostiqué** et un contrat technique a été écrit pour le refermer.
- **Le virage en cours** : passer du simple envoi OSC à un **vrai driver SteamVR** (faux casque/manettes), avec un débogage en conditions réelles déjà bien avancé.
- **L'objectif** : démocratiser la présence virtuelle de qualité — sans matériel cher.

> **Paris, pas promesses.** Le « Stable » du README est un cap, pas encore tout à fait la réalité du terrain — mais les briques difficiles (détection, IK, filtrage, driver) sont là, et le chemin restant est clair.
>
> *Une webcam, ton avatar qui s'anime. C'est tout l'enjeu.*
