# Écosystème Python, Rust ou hybride pour l'appli desktop

Recherche pour l'issue [#5](https://github.com/castorfou/transcripteur/issues/5).
Question : quel écosystème (Python, Rust ou hybride) sert le mieux une appli desktop
de transcription live à l'UI très simple, d'abord sous Linux puis sous Windows, sur
une machine de référence CPU seul (Ryzen 5 7640U) ? Le téléphone est hors périmètre,
mais on veut savoir ce que chaque option permet ou empêche pour un portage futur.

Le vocabulaire suit `CONTEXT.md` : **Source**, **Moteur**, **Enrichissement**,
**Rendu**, **Plugin**, **Session**. L'affichage live est un Rendu toujours actif ; la
fenêtre flottante « toujours au premier plan » en est la forme visée.

Versions relevées le 2026-10-29 ; les tailles de paquets viennent de l'API JSON de
PyPI (`https://pypi.org/pypi/<paquet>/json`).

## Les trois options

- **Python** : tout le programme en Python (UI, orchestration des Plugins, appels aux
  Moteurs via leurs paquets PyPI), distribué par un outil de packaging Python.
- **Rust** : un binaire Rust (UI native ou webview), Moteurs appelés par des crates de
  bindings.
- **Hybride**, deux formes :
    - *cœur Rust, exposé à Python* avec [PyO3](https://pyo3.rs/) et
      [maturin](https://www.maturin.rs/) ;
    - *coque Rust, cœur Python* : [Tauri 2](https://v2.tauri.app/) pour la fenêtre,
      programme Python embarqué comme [sidecar](https://v2.tauri.app/develop/sidecar/).

## 1. Options d'UI et fenêtre « toujours au premier plan »

### Ce que chaque toolkit propose

| Toolkit | Langage | Toujours au premier plan | Source |
|---|---|---|---|
| egui / eframe 0.37 | Rust | `ViewportBuilder::with_always_on_top` / `with_window_level` | [docs.rs egui](https://docs.rs/egui/latest/egui/viewport/struct.ViewportBuilder.html) |
| iced 0.14 | Rust | `window::Level::AlwaysOnTop` | [docs.rs iced](https://docs.rs/iced/latest/iced/window/enum.Level.html) |
| Slint | Rust, C++, Python (bêta), JS | propriété `always-on-top` de `Window`, « on window managers supporting it » | [docs Slint](https://docs.slint.dev/latest/docs/slint/reference/window/window/) |
| Tauri 2 | Rust + front web | `alwaysOnTop` dans `WindowConfig` | [config Tauri](https://v2.tauri.app/reference/config/) |
| PySide6 (Qt 6) | Python | `Qt::WindowStaysOnTopHint` | [doc Qt](https://doc.qt.io/qt-6/qt.html) |
| Flet 1.0 | Python (rendu Flutter) | `page.window.always_on_top`, desktop seulement | [doc Flet Window](https://flet.dev/docs/types/window/) |
| pywebview | Python + front web | paramètre `on_top` de `create_window` | [API pywebview](https://pywebview.flowrl.com/api/) |
| UI web locale (navigateur) | tout langage côté serveur | pas de contrôle de la fenêtre ; seule l'API Document Picture-in-Picture ouvre une fenêtre flottante | [MDN](https://developer.mozilla.org/en-US/docs/Web/API/Document_Picture-in-Picture_API) |

Remarques :

- egui/eframe et iced reposent sur `winit` (dépendances `egui-winit` et `iced_winit`
  dans leurs `Cargo.toml` :
  [eframe](https://github.com/emilk/egui/blob/main/crates/eframe/Cargo.toml),
  [iced](https://github.com/iced-rs/iced/blob/master/Cargo.toml)). Tauri utilise
  `tao` (fork de winit) et `wry` pour la webview
  ([tauri-runtime-wry](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-runtime-wry/Cargo.toml)).
- Tauri s'appuie sur la webview du système : WebKitGTK 4.1 sous Linux
  (`libwebkit2gtk-4.1-dev` sous Debian), WebView2 sous Windows, préinstallée depuis
  Windows 10 1803 ([prérequis Tauri](https://v2.tauri.app/start/prerequisites/)).
- Slint est sous licence GPLv3 ou « Royalty-Free » pour les applications desktop et
  mobiles propriétaires (seul l'embarqué est exclu)
  ([tarifs Slint](https://slint.dev/pricing)). Le paquet Python `slint` est en bêta
  (1.18.1b1) ([PyPI](https://pypi.org/project/slint/)).
- Flet exige Flutter pour construire l'application ; l'appli Flutter embarque le
  programme Python et l'exécute via `serious_python`
  ([publication Flet](https://flet.dev/docs/publish/)).
- Une UI web servie par le programme dans le navigateur ne peut pas se placer
  elle-même au premier plan. L'API Document Picture-in-Picture le permet, mais MDN la
  classe « Limited availability » (non Baseline) et la réserve aux contextes sécurisés
  (HTTPS) ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/Document_Picture-in-Picture_API)).

### La contrainte Wayland s'applique à tous les toolkits

Sous Linux, « toujours au premier plan » dépend du serveur d'affichage, pas du
langage :

- `winit` documente `WindowLevel` ainsi : « iOS / Android / Web / Wayland:
  Unsupported », et précise que le niveau de fenêtre « is just a hint to the OS »
  ([docs.rs winit WindowLevel](https://docs.rs/winit/latest/winit/window/enum.WindowLevel.html)).
  Cela vaut pour egui/eframe et iced.
- Slint applique la propriété seulement là où le gestionnaire de fenêtres la prend en
  charge ([docs Slint](https://docs.slint.dev/latest/docs/slint/reference/window/window/)).
- Qt note que sur certains gestionnaires de fenêtres X11 il faut ajouter
  `Qt::X11BypassWindowManagerHint` ([doc Qt](https://doc.qt.io/qt-6/qt.html)) ; le
  ticket Qt « Always On Top does not work on Linux with Wayland backend »
  ([QTBUG-73456](https://bugreports.qt.io/browse/QTBUG-73456)) est fermé « Out of
  scope ».

Conséquence : sous une session Wayland native, aucune des options ne garantit le
premier plan ; sous X11 ou XWayland, toutes le proposent. Sous Windows, le niveau de
fenêtre n'est pas listé comme non supporté par winit
([docs.rs winit](https://docs.rs/winit/latest/winit/window/enum.WindowLevel.html)).
Le choix d'écosystème ne règle donc pas ce point.

## 2. Packaging et distribution (Linux puis Windows)

### Python

| Outil | Principe | Formats | Source |
|---|---|---|---|
| PyInstaller | gèle l'interpréteur et les dépendances ; `onedir` (dossier, défaut) ou `onefile` (exécutable unique extrait au lancement, démarrage plus lent) | binaire / dossier | [usage PyInstaller](https://pyinstaller.org/en/stable/usage.html) |
| Briefcase (BeeWare) | empaquette une appli Python en appli native | Linux : AppImage, Flatpak, paquet système ; Windows : appli Windows, projet Visual Studio ; Android (Gradle), iOS (Xcode), web | [plateformes Briefcase](https://briefcase.beeware.org/en/stable/reference/platforms/index.html) |
| Nuitka | compile le Python en C puis en binaire (GCC, Clang, MSVC, Zig) ; modes `standalone`, `onefile`, `app` | binaire / dossier | [manuel Nuitka](https://nuitka.net/user-documentation/user-manual.html) |
| Flet | `flet build` via Flutter | apk/aab, ipa, linux, windows, macos, web | [publication Flet](https://flet.dev/docs/publish/) |

PyInstaller ne construit pas pour un autre OS : « you must install PyInstaller on each
platform and bundle your app separately on each »
([usage PyInstaller](https://pyinstaller.org/en/stable/usage.html)). Il faut donc un
runner Windows en CI pour la version Windows.

### Rust

- Tauri produit `deb`, `rpm`, `appimage` (Linux), `nsis`, `msi` (Windows), `app`,
  `dmg` (macOS) ([config Tauri, `bundle.targets`](https://v2.tauri.app/reference/config/)),
  et documente la distribution vers Snap, Flatpak, AUR, Microsoft Store, Google Play
  et App Store ([distribution Tauri](https://v2.tauri.app/distribute/)). Une appli
  Tauri minimale peut faire moins de 600 Ko, la webview n'étant pas embarquée
  ([Tauri, présentation](https://v2.tauri.app/start/)).
- egui, iced et Slint produisent un exécutable `cargo build --release` ; l'installeur
  ou l'AppImage relèvent d'outils séparés (non couverts ici).
- `x86_64-unknown-linux-gnu` et `x86_64-pc-windows-msvc` sont des cibles Tier 1
  « guaranteed to work » ([support des plateformes rustc](https://doc.rust-lang.org/rustc/platform-support.html)).

### Taille : les runtimes pèsent plus que le langage

Tailles des wheels Python cp312 x86_64 (Linux / Windows), d'après PyPI :

| Paquet | Version | Linux | Windows |
|---|---|---|---|
| `torch` (pour pyannote.audio) | 2.15.0 | 539 Mo | 127 Mo |
| `pyside6-essentials` + `pyside6-addons` | 6.12.0 | 84 + 59 Mo | 82 + 41 Mo |
| `ctranslate2` (pour faster-whisper) | 4.8.4 | 43 Mo | 20 Mo |
| `onnxruntime` | 1.31.1 | 24 Mo | 15 Mo |
| `pywhispercpp` | 1.5.1 | 4,7 Mo | 1,4 Mo |
| `sherpa-onnx` | 2.0.1 | 4,3 Mo | 2,2 Mo |

La wheel `torch` Linux de PyPI cible CUDA ; les wheels CPU seules sont sur
`https://download.pytorch.org/whl/cpu`
([guide uv × PyTorch](https://docs.astral.sh/uv/guides/integration/pytorch/)). Les
modèles des Moteurs s'ajoutent quel que soit l'écosystème. Un binaire Rust qui charge
ONNX Runtime ou CTranslate2 embarque aussi leurs bibliothèques natives (voir §3) ; la
différence porte sur l'interpréteur Python et les paquets gelés, pas sur les runtimes.

## 3. Accès aux runtimes des Moteurs

| Runtime | Python | Rust | Statut des bindings Rust |
|---|---|---|---|
| ONNX Runtime | `onnxruntime` (officiel) | [`ort`](https://ort.pyke.io/) 2.0.0-rc.14 | tiers (pyke) ; binaires ONNX Runtime fournis par défaut, ou `load-dynamic` / lien statique sur un build maison ([linking ort](https://ort.pyke.io/setup/linking)) |
| CTranslate2 | `ctranslate2` (officiel), [faster-whisper](https://github.com/SYSTRAN/faster-whisper) | [`ct2rs`](https://github.com/jkawamoto/ctranslate2-rs) 0.10.2 | tiers ; compile CTranslate2 avec CMake (ou feature `system`), OpenBLAS via vcpkg sous Windows |
| whisper.cpp | `pywhispercpp` et autres bindings | [`whisper-rs`](https://codeberg.org/tazz4843/whisper-rs) | tiers ; dépôt GitHub archivé le 30/07/2025, développement poursuivi sur Codeberg ([GitHub whisper-rs](https://github.com/tazz4843/whisper-rs)) |
| sherpa-onnx | `sherpa-onnx` (officiel) | crate `sherpa-onnx` | **officiel**, maintenu dans le dépôt sherpa-onnx ([API Rust sherpa-onnx](https://k2-fsa.github.io/sherpa/onnx/rust-api/index.html)) |

Détails :

- CTranslate2 expose officiellement des API Python et C++ ; il gère Whisper, NLLB et
  M2M-100, l'INT8 et la sélection AVX/AVX2 au lancement
  ([CTranslate2](https://github.com/OpenNMT/CTranslate2)). faster-whisper s'appuie
  dessus, propose `compute_type="int8"` sur CPU, intègre Silero VAD et décode l'audio
  sans FFmpeg système grâce à PyAV ([faster-whisper](https://github.com/SYSTRAN/faster-whisper)).
- whisper.cpp est en C/C++ sans dépendance externe, avec intrinsèques AVX, NEON et
  quantification ; il fournit un exemple `whisper-stream` de transcription micro en
  temps réel et cite whisper-rs (Rust) et pywhispercpp (Python) parmi ses bindings
  ([whisper.cpp](https://github.com/ggml-org/whisper.cpp)).
- sherpa-onnx propose des API C, C++, Python, Rust, Kotlin, Swift, Go, C#, Dart et
  JavaScript, sur Linux, Windows, macOS, Android et iOS, avec ASR en streaming, VAD,
  identification de langue et diarisation
  ([sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)).

Bilan : Python a des bindings officiels pour les quatre runtimes. Rust a un binding
officiel pour sherpa-onnx et des bindings tiers pour les trois autres.

## 4. Écosystème pour les Enrichissements futurs

### Détection d'interlocuteurs (diarisation)

- [pyannote.audio](https://github.com/pyannote/pyannote-audio) (MIT) est en Python sur
  PyTorch. Le pipeline local `speaker-diarization-community-1` demande d'accepter ses
  conditions sur Hugging Face et un jeton d'accès ; le README ne publie pas de chiffres
  CPU, ses mesures de vitesse viennent d'un GPU H100.
- [pyannote-rs](https://github.com/thewh1teagle/pyannote-rs) (MIT, tiers) exécute en
  Rust les modèles `segmentation-3.0` et `wespeaker-voxceleb-resnet34-LM` via ONNX
  Runtime ; il annonce une heure d'audio traitée en moins d'une minute sur CPU
  (chiffre du projet, non vérifié ici).
- sherpa-onnx inclut une diarisation hors ligne, disponible depuis Python comme depuis
  Rust ([API Rust sherpa-onnx](https://k2-fsa.github.io/sherpa/onnx/rust-api/index.html)).

### Traduction

- CTranslate2 exécute NLLB et M2M-100 ([CTranslate2](https://github.com/OpenNMT/CTranslate2)).
  En Python : paquet officiel, et [Argos Translate](https://github.com/argosopentech/argos-translate)
  (bibliothèque de traduction hors ligne, MIT/CC0, sur OpenNMT et CTranslate2). En
  Rust : `ct2rs` gère Marian-MT, NLLB, T5 et BART, avec une API de streaming
  ([ct2rs](https://github.com/jkawamoto/ctranslate2-rs)).

Bilan : la référence de la diarisation (pyannote.audio) est Python et tire PyTorch.
Les alternatives ONNX (sherpa-onnx, pyannote-rs) sont accessibles depuis les deux
écosystèmes.

## 5. Portage mobile futur

| Option | Ce qui existe côté mobile |
|---|---|
| Python | CPython 3.13 prend en charge Android ([PEP 738](https://peps.python.org/pep-0738/)) et iOS ([PEP 730](https://peps.python.org/pep-0730/)) au Tier 3. Python y est embarqué dans l'appli ; la doc officielle oriente vers Briefcase, Buildozer, Chaquopy, Flet, pyqtdeploy ([Python sur Android](https://docs.python.org/3/using/android.html)). Flet et Briefcase construisent apk/ipa ([Flet](https://flet.dev/docs/publish/), [Briefcase](https://briefcase.beeware.org/en/stable/reference/platforms/index.html)). Les paquets à extensions natives doivent avoir des wheels Android/iOS (cibuildwheel recommandé pour les produire, [Python sur Android](https://docs.python.org/3/using/android.html)). |
| Rust | `aarch64-linux-android` et `aarch64-apple-ios` sont Tier 2 « guaranteed to build » ([rustc](https://doc.rust-lang.org/rustc/platform-support.html)). eframe cible Android mais ne cite pas iOS ([eframe](https://github.com/emilk/egui/tree/main/crates/eframe)) ; Tauri 2 cible Android et iOS ([Tauri](https://v2.tauri.app/start/)), iOS exigeant macOS/Xcode ([prérequis](https://v2.tauri.app/start/prerequisites/)) ; Slint documente Android ([Slint Android](https://docs.slint.dev/latest/docs/slint/guide/platforms/android)). |
| Hybride Tauri + sidecar Python | Sur Android et iOS, le plugin shell de Tauri « Only allows to open URLs via `open` » ([plugin shell](https://v2.tauri.app/plugin/shell/)) : un sidecar Python ne peut pas y être lancé. Le portage mobile demanderait de réécrire le cœur. |
| Hybride cœur Rust + PyO3 | Le cœur Rust se recompile pour les cibles mobiles ; la partie Python suit les contraintes de la ligne Python. |

Côté Moteurs, sherpa-onnx (API Kotlin/Swift, Android/iOS) et whisper.cpp
(Android/iOS) visent déjà le mobile ([sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx),
[whisper.cpp](https://github.com/ggml-org/whisper.cpp)), indépendamment du langage de
l'appli desktop.

## 6. Outillage du dépôt et CI

### État actuel

- `pyproject.toml` : build `setuptools`, paquets sous `src/`, `ruff`, `mypy`,
  `pytest`, MkDocs et extras `dev`.
- `.pre-commit-config.yaml` : `ruff`, `ruff-format`, `mypy` (sur `src/`),
  `detect-secrets`, hooks génériques, `nbdev-clean`, et
  `check-added-large-files` à 90 Mo (à garder en tête pour les modèles).
- `.devcontainer/devcontainer.json` : features `uv`, `github-cli`, `git`, `node`
  (LTS), `docker-in-docker` ; venv dans `/tmp/venv`.
- `.github/workflows/ci.yml` : jobs `tests` (uv, ruff, pytest, mypy), `pre-commit`
  (avec `uv lock --check`) et `build` (`uv build`), tous sur `ubuntu-latest` ;
  `docs.yml` : MkDocs vers GitHub Pages.

### Ce que Rust changerait

| Domaine | Python seul | Avec Rust |
|---|---|---|
| Devcontainer | inchangé | ajouter la feature `ghcr.io/devcontainers/features/rust:1` (rustup, `rustfmt`, `clippy`, `rust-analyzer` par défaut) ([feature Rust](https://github.com/devcontainers/features/tree/main/src/rust)) ; pour Tauri, les dépendances système WebKitGTK ([prérequis Tauri](https://v2.tauri.app/start/prerequisites/)) ; un affichage pour lancer l'UI depuis le conteneur (vrai aussi pour une UI Python native) |
| Build Python | `setuptools` | pour PyO3 : backend `maturin` (`uv init --build-backend maturin` génère `Cargo.toml` et `lib.rs`) ([uv init](https://docs.astral.sh/uv/concepts/projects/init/)) ; le backend `uv_build` ne gère que le Python pur ([uv build backend](https://docs.astral.sh/uv/concepts/build-backend/)) ; wheels Linux redistribuables via manylinux2014 ou zig ([maturin](https://www.maturin.rs/)) |
| Lint / format | `ruff` | `cargo fmt`, `cargo clippy` en plus, à ajouter dans pre-commit (hooks locaux) |
| Tests | `pytest` | `cargo test` en plus de `pytest` (hybride) ou à sa place (Rust) |
| CI | `astral-sh/setup-uv` | `dtolnay/rust-toolchain@stable` avec `components: clippy, rustfmt` ([rust-toolchain](https://github.com/dtolnay/rust-toolchain)) ; runner `windows-latest` pour les binaires Windows, comme pour PyInstaller |
| Front (Tauri) | — | une base de code HTML/JS ; Node.js est déjà dans le devcontainer |
| Documentation | MkDocs | MkDocs reste possible pour la doc utilisateur ; `cargo doc` pour l'API Rust |

## Ce que ça implique pour la décision

### Tableau comparatif

| Critère | Python | Rust | Hybride (Tauri + sidecar Python, ou cœur Rust + PyO3) |
|---|---|---|---|
| UI simple + toujours au premier plan | PySide6, Flet, pywebview le proposent | egui, iced, Slint, Tauri le proposent | Tauri (webview) ; ou UI Python au-dessus d'un cœur Rust |
| Limite Wayland | non garanti sous Wayland natif (Qt : « Out of scope ») | non supporté par winit sous Wayland | identique : dépend de la coque UI |
| Accès aux Moteurs | bindings officiels pour ONNX Runtime, CTranslate2, sherpa-onnx ; faster-whisper | `ort`, `ct2rs`, `whisper-rs` tiers ; sherpa-onnx officiel | Tauri+sidecar : comme Python ; PyO3 : au choix par Moteur |
| Enrichissements | pyannote.audio (référence, PyTorch), Argos Translate, CTranslate2 | pyannote-rs (tiers), sherpa-onnx, `ct2rs` | comme Python pour le sidecar |
| Packaging | PyInstaller / Nuitka / Briefcase / Flet, un build par OS | binaire unique ; Tauri : deb, rpm, AppImage, msi, nsis | deux chaînes (cargo/Tauri + PyInstaller pour le sidecar, nommé par triple cible) |
| Taille hors modèles | interpréteur + paquets gelés ; PyTorch domine si pyannote.audio | runtimes natifs seulement ; Tauri < 600 Ko sans webview embarquée | somme des deux pour Tauri + sidecar |
| Mobile futur | Python embarqué Tier 3 (3.13), Flet/Briefcase ; wheels mobiles à vérifier par Moteur | Tauri Android/iOS, egui Android, Slint Android ; Rust Tier 2 | Tauri + sidecar : sidecar impossible sur mobile ; PyO3 : cœur Rust portable |
| Outillage du dépôt | déjà en place | feature Rust, clippy/fmt, cargo test, CI Rust | les deux outillages à la fois |

### Points de bascule

- **Wayland natif est-il une cible de l'affichage flottant ?** Si oui, aucun
  écosystème ne le garantit : la question se règle par une solution de contournement
  (XWayland, règle du compositeur, réglage manuel par l'utilisateur), pas par le
  langage.
- **pyannote.audio est-il l'Enrichissement de diarisation visé ?** Si oui, PyTorch
  entre dans l'appli et Python (ou un sidecar Python) devient difficile à éviter. Si
  une diarisation ONNX (sherpa-onnx, pyannote-rs) suffit, Rust reste ouvert.
- **Quel runtime pour le premier Moteur ?** faster-whisper (CTranslate2) n'a pas de
  binding Rust officiel ; sherpa-onnx en a un ; whisper.cpp dépend d'un binding tiers
  dans les deux langages.
- **Poids accordé au portage mobile.** Un cœur Rust ou des Moteurs déjà mobiles
  (sherpa-onnx, whisper.cpp) gardent la porte ouverte ; Tauri + sidecar Python la
  ferme pour la partie Python.
- **Coût d'outillage accepté.** Python garde l'outillage existant ; Rust ou l'hybride
  ajoutent une toolchain, des hooks et une CI Rust, et pour l'hybride deux chaînes de
  packaging.
- **Taille de distribution.** Elle dépend surtout des runtimes et des modèles
  choisis, plus que du langage ; l'écart se joue sur l'interpréteur Python gelé et
  sur PyTorch.
