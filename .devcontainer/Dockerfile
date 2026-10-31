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

# Compilation des crates natives Rust : whisper-rs (cmake, libclang pour bindgen)
# et cpal (en-têtes ALSA)
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake clang libclang-dev pkg-config libasound2-dev \
    && rm -rf /var/lib/apt/lists/*
