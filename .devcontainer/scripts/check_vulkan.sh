#!/usr/bin/env bash
# Vérifie que le conteneur voit l'iGPU par Vulkan (#23) : affiche le pilote et le périphérique.
# Usage : sg render-hote -c 'bash .devcontainer/scripts/check_vulkan.sh'
set -euo pipefail

NOEUD="${NOEUD_RENDU:-/dev/dri/renderD128}"
if [ ! -r "$NOEUD" ]; then
    echo "❌ $NOEUD inaccessible : relancer avec sg render-hote -c, ou ensure_render_group.sh"
    exit 1
fi

# Sans DISPLAY : vulkaninfo tenterait d'ouvrir une fenêtre sur un serveur X joignable ou non
RESUME="$(env -u DISPLAY -u WAYLAND_DISPLAY vulkaninfo --summary 2>&1)"
echo "$RESUME" | grep -E "deviceName|driverName|driverInfo|apiVersion"
if ! echo "$RESUME" | grep -q "PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU"; then
    echo "❌ Aucun iGPU vu par Vulkan (llvmpipe ne compte pas)"
    exit 1
fi
echo "✅ iGPU accessible par Vulkan"
