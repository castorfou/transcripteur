#!/usr/bin/env bash
# Vérifie que le conteneur entend l'audio de l'hôte (#7) : enregistre quelques secondes du micro
# par défaut puis du Son système (monitor du sink par défaut) et affiche le niveau crête de chacun.
# Usage : bash .devcontainer/scripts/check_audio.sh [durée en secondes, défaut 5]
set -euo pipefail

DUREE="${1:-5}"
SORTIE="${TMPDIR:-/tmp}/check_audio"
mkdir -p "$SORTIE"

if ! pactl info >/dev/null 2>&1; then
    echo "❌ Pas de serveur audio joignable (PULSE_SERVER=${PULSE_SERVER:-non défini})"
    echo "   Vérifier sur l'hôte : ls \"\$XDG_RUNTIME_DIR/pulse/native\" puis reconstruire le conteneur"
    exit 1
fi
pactl info | grep -E "^(Nom du serveur|Server Name|Source par défaut|Default Source|Destination par défaut|Default Sink)"

crete() {
    python3 - "$1" <<'PY'
import array, sys, wave
with wave.open(sys.argv[1]) as w:
    echantillons = array.array("h", w.readframes(w.getnframes()))
pic = max((abs(e) for e in echantillons), default=0) / 32768
print(f"crête {pic:.3f} ({'silence' if pic < 0.01 else 'signal'})")
PY
}

enregistrer() {
    local nom="$1" appareil="$2" fichier="$SORTIE/$1.wav"
    echo "🎙️  $nom : ${DUREE} s depuis $appareil…"
    timeout "$DUREE" parecord --device="$appareil" --rate=16000 --channels=1 \
        --format=s16le --file-format=wav "$fichier" || true
    echo "   $fichier : $(crete "$fichier")"
}

read -rp "Entrée, puis parler dans le micro… "
enregistrer micro @DEFAULT_SOURCE@
# Le monitor ne capte que la sortie par défaut : la vidéo doit y jouer (pactl list sink-inputs)
read -rp "Lancer une vidéo sur l'hôte, sur la sortie par défaut, puis Entrée… "
enregistrer son_systeme @DEFAULT_MONITOR@
echo "Réécouter : paplay $SORTIE/micro.wav ; paplay $SORTIE/son_systeme.wav"
