# Prototype jetable : choix du toolkit d'UI Rust

Répond au ticket « Choix du toolkit d'UI Rust ». Trois fois la même petite fenêtre, avec egui,
iced et Slint, nourrie par un flux simulé dans un thread de fond et 1500 énoncés d'historique
(1 h de Session). Ce code ne sera jamais mergé.

## Lancer sur l'hôte

```bash
cargo build --release          # dans le devcontainer
./lancer.sh mesures            # sur l'hôte : scénario automatique, résultats dans resultats.txt
./lancer.sh egui               # sur l'hôte : à la main, Wayland natif
./lancer.sh egui x11           # sur l'hôte : à la main, XWayland
```

## Scénario automatique

`--auto` : 15 s de live seul, puis 30 s de défilement continu de l'historique, puis fermeture.
Il note la latence du tick à la frame, l'intervalle entre frames, le CPU, la RAM et la mémoire
GPU du processus.

## À vérifier à la main

1. Fond à 0 % : voit-on le bureau sans halo, le texte reste-t-il net et opaque ?
2. Déplacer par l'en-tête, redimensionner par les huit bords et coins.
3. `Alt+Espace` sur la fenêtre sans barre : le menu GNOME propose-t-il « Toujours au premier
   plan » ? Raccourci dédié : `gsettings set org.gnome.desktop.wm.keybindings toggle-above "['<Super>t']"`.
4. Déplier : le bas de la fenêtre reste-t-il en place ?
5. Molette dans l'historique pendant le live : ça saccade ?
6. A−/A+ de chaque zone : indépendants, sans à-coup ?
7. Live : provisoire en italique atténué, mots modifiés surlignés, accents.

## Constats déjà établis, sans écran

- Sous Wayland, winit refuse de lire et d'écrire la position de la fenêtre. Les trois toolkits
  reposent sur winit : aucun ne tient le bas fixe au dépliage en Wayland natif.
- egui rend la forme sans écart. iced aussi, à condition d'embarquer sa police.
- Slint ne sait pas surligner un mot : `StyledText` n'a pas de fond par span, le mot modifié
  est ici en couleur soulignée.
- Slint fournit `WindowMoveArea` et `resize-border-width` ; egui et iced demandent d'écrire
  les poignées de bord.

## Essai Slint

Disposition de la forme validée : en-tête, historique, live, commandes. Le live a une hauteur
fixe, texte calé en bas ; il suit le direct tant qu'on n'a pas remonté l'historique, avec barre
orange et pastille « Revenir au direct » sinon.
