# Le live tourne sur CPU avec Nemotron, le Texte définitif sur iGPU avec Whisper turbo

Le Texte provisoire vient de Nemotron 3.5 ASR Streaming 0.6B (fenêtre de 320 ms), sur CPU. Le Texte définitif vient d'une seconde passe par Énoncé avec Whisper turbo q5 (`audio_ctx` 1000), sur l'iGPU Radeon 760M par Vulkan. Silero VAD découpe les Énoncés, et Whisper tiny, restreint à EN/DE/FR, détecte la Langue parlée. Ce couple obtient la meilleure lecture humaine en Son système, sur une vraie vidéo (3/3), tout en laissant la machine utilisable pendant une Session.

## Considered Options

- **Nemotron + Parakeet TDT 0.6B v3, tout sur CPU** : garde un outil sans GPU, avec un Texte définitif plus rapide (1,4 s en p95), mais la lecture humaine descend à 2/3 et le français reste loin derrière l'allemand et l'anglais.
- **Nemotron + Canary** : écarté, plus mauvais que les deux autres en français et en allemand.
- **Whisper turbo sur CPU** : écarté, trop lent pour le temps réel sur le Ryzen 5 7640U (RTF de 2,3 à 3,3).
- **`audio_ctx` 750** : plus rapide, mais la qualité s'effondre. 1000 est le meilleur compromis mesuré.

## Consequences

- L'outil exige un GPU compatible avec Vulkan pour son Texte définitif. Le repli sur une machine sans un tel GPU reste à décider.
- Les seuils sont relevés : p95 < 5 s pour le Texte définitif (la lecture prime sur la latence), et 2,6 Go de mémoire GPU comprise, à revoir avec l'UI.
- `sherpa-onnx` est lié en feature `shared`. En statique, il entre en conflit à l'édition de liens avec whisper.cpp. Le packaging devra livrer ses bibliothèques partagées.
- La Langue parlée vient d'un détecteur séparé, parce que la crate `sherpa-onnx` n'expose pas l'étiquette de langue de Nemotron. Whisper tiny se trompe sur environ 1,7 % des Énoncés, et peut alors forcer Whisper turbo dans la mauvaise langue.
- ggml exige des en-têtes Vulkan récents et un accès au nœud de rendu, voir `docs/dev/environnement-rust.md`.
