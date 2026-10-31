#!/usr/bin/env bash
# PROTOTYPE jetable. À lancer sur l'HÔTE, dans une session GNOME Wayland.
#   ./lancer.sh egui|iced|slint [wayland|x11]   essai à la main
#   ./lancer.sh mesures                         toutes les mesures (≈ 15 min), dans resultats.txt
cd "$(dirname "$0")"
BIN=target/release

lance() { # toolkit session [args...]
    local t=$1 s=$2; shift 2
    if [ "$s" = x11 ]; then
        env -u WAYLAND_DISPLAY "$BIN/essai-$t" "$@"
    else
        "$BIN/essai-$t" "$@"
    fi
}

case "${1:-}" in
    egui|iced|slint) lance "$1" "${2:-wayland}" ;;
    mesures)
        : > resultats.txt
        for s in wayland x11; do
            for v in glow wgpu; do EGUI_RENDERER=$v lance egui $s --auto | grep RESULTAT | tee -a resultats.txt; done
            for v in wgpu tiny-skia; do ICED_BACKEND=$v lance iced $s --auto | grep RESULTAT | tee -a resultats.txt; done
            for v in winit-femtovg winit-software; do SLINT_BACKEND=$v lance slint $s --auto | grep RESULTAT | tee -a resultats.txt; done
        done
        ;;
    *) sed -n 2,4p "$0"; exit 1 ;;
esac
