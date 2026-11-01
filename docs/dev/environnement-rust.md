# Environnement Rust et Vulkan

Le devcontainer fournit tout ce qu'il faut pour compiler et lancer les Moteurs retenus : Nemotron sur CPU et Whisper turbo sur l'iGPU, voir `docs/adr/0004-moteurs-nemotron-whisper-igpu.md`.

## Contenu de l'image

| Élément | Rôle | Source |
| --- | --- | --- |
| Rust stable, `rustfmt`, `clippy` | Compilation | Feature `rust` du devcontainer |
| `cmake`, `clang`, `libclang-dev`, `pkg-config` | Compilation de whisper.cpp et bindgen | `Dockerfile` |
| `libasound2-dev` | Compilation de `cpal` | `Dockerfile` |
| `libvulkan-dev`, `glslc` | Backend Vulkan de ggml et shaders | `Dockerfile` |
| `mesa-vulkan-drivers`, `vulkan-tools` | Pilote RADV (Mesa 22.3) et `vulkaninfo` | `Dockerfile` |
| Vulkan-Headers 1.4.309 | En-têtes dans `/usr/local/include` | `Dockerfile`, empreinte figée |

Les en-têtes de Debian bookworm (1.3.239) ne suffisent pas : ggml s'arrête à la compilation sur `'LayerSettingEXT' is not a member of 'vk'`. Les en-têtes de `/usr/local/include` passent avant ceux de `/usr/include`, sans variable d'environnement.

Les mesures du Spike utilisent Mesa 22.3. Changer de version de Mesa peut changer la latence du Texte définitif.

## Accès à l'iGPU

Le nœud `/dev/dri/renderD128` est visible parce que la feature `docker-in-docker` lance le conteneur en mode privilégié. Il garde le gid de l'hôte, qui varie d'un hôte à l'autre. À chaque démarrage, `.devcontainer/scripts/ensure_render_group.sh` crée le groupe `render-hote` avec ce gid et y ajoute l'utilisateur. Sans nœud, le conteneur démarre quand même, sans iGPU.

Le serveur VS Code démarre avant ce script : ses terminaux, et tout ce qu'ils lancent, ne voient donc pas le groupe, même après un rebuild. Toute commande qui utilise l'iGPU depuis VS Code se préfixe par `sg` :

```bash
sg render-hote -c 'bash .devcontainer/scripts/check_vulkan.sh'
```

`check_vulkan.sh` affiche les périphériques Vulkan et échoue si aucun iGPU n'est visible. `llvmpipe`, le rendu logiciel de Mesa, apparaît aussi dans la liste et ne compte pas.

## Compiler et lancer

La crate `sherpa-onnx` est liée en feature `shared` : en statique, elle entre en conflit à l'édition de liens avec whisper.cpp. Le Vulkan de whisper.cpp s'active par la feature `vulkan` de `whisper-rs`.

```toml
[dependencies]
sherpa-onnx = { version = "2.0.1", default-features = false, features = ["shared"] }
whisper-rs = { version = "0.16", features = ["vulkan"] }
```

La compilation télécharge les bibliothèques de `sherpa-onnx` et les dépose à côté du binaire. Elles ne sont pas dans son chemin de recherche, il faut donc les indiquer au lancement :

```bash
sg render-hote -c 'LD_LIBRARY_PATH=target/release target/release/<binaire>'
```

## Limites

- `.devcontainer/` vient du template PyFoundry : un `/cruft-update` peut entrer en conflit avec ces lignes.
- La CI ne compile pas de Rust, et aucun test automatisé n'utilise l'iGPU.
