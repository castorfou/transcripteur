#!/usr/bin/env bash
# Donne à l'utilisateur l'accès à l'iGPU (#23) : le nœud de rendu garde le gid de l'hôte, qui varie
# d'un hôte à l'autre et n'existe pas dans l'image. Appelé par postStartCommand à chaque démarrage.
# Un processus déjà lancé ne voit pas ce groupe : le préfixer par `sg render-hote -c`.
# Sans iGPU, le conteneur démarre quand même.
set -euo pipefail

NOEUD="${NOEUD_RENDU:-/dev/dri/renderD128}"
if [ ! -e "$NOEUD" ]; then
    echo "⚠️  Pas de $NOEUD : le Texte définitif sur iGPU est indisponible dans ce conteneur"
    exit 0
fi

GID_NOEUD="$(stat -c %g "$NOEUD")"
UTILISATEUR="$(id -un)"
if id -G "$UTILISATEUR" | tr ' ' '\n' | grep -qx "$GID_NOEUD"; then
    exit 0
fi

# getent sort en erreur quand le gid n'existe pas, le cas normal d'un conteneur neuf
GROUPE="$(getent group "$GID_NOEUD" | cut -d: -f1 || true)"
if [ -z "$GROUPE" ]; then
    GROUPE=render-hote
    sudo groupadd -g "$GID_NOEUD" "$GROUPE"
fi
sudo usermod -aG "$GROUPE" "$UTILISATEUR"
echo "✅ $UTILISATEUR ajouté au groupe $GROUPE ($NOEUD)"
