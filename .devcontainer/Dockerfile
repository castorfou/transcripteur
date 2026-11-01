FROM mcr.microsoft.com/devcontainers/python:1-3.12-bookworm

# Supprimer la source Yarn cassée héritée de l'image de base
RUN rm -f /etc/apt/sources.list.d/yarn.list \
    /usr/share/keyrings/yarnkey.gpg \
    /etc/apt/sources.list.d/yarn.list.save 2>/dev/null || true

# Créer le répertoire cache avec les bons droits avant le montage du volume uv
# Sans ça, Docker crée /home/vscode/.cache en root lors du mount, ce qui bloque pre-commit
RUN mkdir -p /home/vscode/.cache && chown vscode:vscode /home/vscode/.cache

# Audio de l'hôte (#7) : clients PulseAudio (pactl, parecord) pour parler au PipeWire de l'hôte
# via le socket monté (voir mounts et PULSE_SERVER dans devcontainer.json), et ALSA redirigé
# vers Pulse pour que les bibliothèques ALSA (cpal, PortAudio) passent aussi par l'hôte
# plutôt que d'ouvrir /dev/snd directement
RUN apt-get update \
    && apt-get install -y --no-install-recommends pulseaudio-utils libasound2-plugins \
    && rm -rf /var/lib/apt/lists/* \
    && printf 'pcm.!default pulse\nctl.!default pulse\n' > /etc/asound.conf

# Compilation des crates natives Rust : whisper-rs (cmake, libclang pour bindgen), cpal (en-têtes
# ALSA). Texte définitif sur iGPU (#23) : chargeur Vulkan et glslc pour ggml, pilote RADV de Mesa
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake clang libclang-dev pkg-config \
        libasound2-dev libvulkan-dev glslc mesa-vulkan-drivers vulkan-tools \
    && rm -rf /var/lib/apt/lists/*

# ggml n'accepte pas les en-têtes Vulkan de bookworm (1.3.239) : version figée dans
# /usr/local/include, qui passe avant /usr/include
RUN curl -fsSL https://github.com/KhronosGroup/Vulkan-Headers/archive/refs/tags/v1.4.309.tar.gz \
        -o /tmp/vulkan-headers.tgz \
    && echo "437925ada160d86ed763d29dcb9318c1bb0d024d7deaf77bc7c170b8eb6b6f10  /tmp/vulkan-headers.tgz" \
        | sha256sum -c - \
    && tar xzf /tmp/vulkan-headers.tgz --strip-components=2 -C /usr/local/include \
        Vulkan-Headers-1.4.309/include \
    && rm /tmp/vulkan-headers.tgz
