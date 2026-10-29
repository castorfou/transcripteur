#!/usr/bin/env bash
# Lancé sur l'HÔTE par initializeCommand, avant check_mounts.sh (#7) : fait pointer
# ~/.devcontainer/host-pulse vers le dossier du socket PipeWire-Pulse de l'hôte, monté sur
# /run/host-pulse. Passer par le HOME évite ${localEnv:XDG_RUNTIME_DIR}, que check_mounts.sh ne
# résout pas et qui peut manquer à l'environnement de VS Code. Sans serveur audio sur l'hôte,
# un dossier vide le remplace : le conteneur démarre, sans audio.
set -euo pipefail

LIEN="$HOME/.devcontainer/host-pulse"
DOSSIER="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/pulse"

mkdir -p "$(dirname "$LIEN")"
if [[ -S "$DOSSIER/native" ]]; then
    # Un ancien dossier de repli vide laisse la place au lien
    [[ -d "$LIEN" && ! -L "$LIEN" ]] && rmdir "$LIEN"
    ln -sfn "$DOSSIER" "$LIEN"
    echo "Audio de l'hôte : $LIEN -> $DOSSIER"
else
    [[ -L "$LIEN" ]] && rm "$LIEN"
    mkdir -p "$LIEN"
    echo "Audio de l'hôte : pas de socket $DOSSIER/native, conteneur sans audio"
fi
