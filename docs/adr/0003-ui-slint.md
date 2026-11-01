# L'UI est écrite avec Slint, en Wayland natif

L'affichage live est écrit avec Slint, sur son rendu GPU femtovg, et tourne en Wayland natif. Slint reproduit la forme validée avec le moins de code : déplacement et redimensionnement fournis, liste virtualisée à hauteurs variables, composants et thème propres à l'outil. Sur l'hôte de référence, il est aussi le plus sobre en mémoire : 154 Mo de RAM et de mémoire GPU, contre 162 Mo pour egui et 194 Mo pour iced.

## Considered Options

- **egui** (glow) : rendu fidèle, surlignage compris, et CPU le plus bas en défilement (11 %, contre 18 % pour Slint). Écarté parce que la virtualisation à hauteurs variables et les poignées de bord sont à écrire à la main.
- **iced** : pas de liste virtualisée, et un plantage au démarrage si la police par défaut manque. Son rendu logiciel tiny-skia coûte 38 % de CPU en défilement sous Wayland, 99 % sous XWayland.
- **Tauri** : une interface web ajouterait une seconde technologie embarquée, contre l'ADR `0001-stack-rust`.
- **XWayland** : seule voie possible pour positionner la fenêtre, mais technologie en perte de vitesse, avec une latence du texte live plus élevée (23 ms contre 2 ms en médiane, à l'affichage d'un nouveau texte).

## Consequences

- Sous Wayland, une application ne peut ni lire ni fixer la position de sa fenêtre. Au dépliage, l'historique s'étend donc vers le bas : le bas de la fenêtre ne reste pas fixe.
- `StyledText` n'a ni fond par portion de texte, ni taille ou police propre à une portion. Les mots modifiés sont en couleur soulignée, et l'horodatage du live a la taille du texte.
- Le déplacement passe par `drag_window` de winit dès l'appui, suivi d'événements de relâchement et de sortie envoyés à Slint. Le compositeur garde le relâchement : sans ces événements, la `TouchArea` conserverait la souris et capterait tous les clics suivants. `WindowMoveArea` n'est pas utilisée, parce qu'elle demande un second clic sur une fenêtre sans focus.
- Le contraste sur fond clair repose sur une copie sombre du texte live, décalée d'un pixel : `StyledText` n'a ni contour ni ombre.
- Le binaire distribué relève de la licence Slint Royalty-free 2.0 : il doit afficher `AboutSlint` dans son écran « À propos », ou le badge d'attribution figurer sur une page publique, comme le site de documentation. Le code du dépôt reste sous licence MIT.
- Les polices sont embarquées (Atkinson Hyperlegible et JetBrains Mono, sous licence OFL), l'outil ne dépend donc pas de celles du système.
- Le portage Windows n'est pas vérifié : winit et femtovg y fonctionnent, mais la fenêtre sans barre, la transparence et « toujours au premier plan » avec PowerToys restent à essayer.
