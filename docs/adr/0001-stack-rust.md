# L'outil livré est écrit en Rust ; Python reste pour la doc et l'analyse

L'outil livré est écrit entièrement en Rust : Sources (cpal + rubato), Moteurs, Session, Enrichissements, Rendus et UI. On veut une seule technologie embarquée, multi-plateforme (Linux puis Windows), sûre et rapide, et ce projet sert aussi à apprendre Rust. Python reste dans le dépôt pour MkDocs, le scoring du Banc d'essai et les notebooks, sans jamais être livré.

## Considered Options

- **Python pur** : itération la plus rapide et outillage déjà en place, mais aucune bibliothèque Python ne capte de façon fiable le micro et le Son système sous Linux et Windows à la fois.
- **Appli Python + extension Rust limitée aux Sources** (PyO3/maturin) : Rust là où Python est faible, et itération rapide ailleurs. Écartée parce qu'elle garde deux technologies embarquées et l'interpréteur Python à distribuer.
- **Cœur Rust exposé à Python** : la frontière PyO3 est large à maintenir, sans gain par rapport au Rust pur une fois le Banc d'essai découpé comme ci-dessous.

## Consequences

- On accepte une itération plus lente pendant la phase de mesure, au profit d'un seul langage dans le produit.
- Le Banc d'essai est découpé : un binaire de dev en Rust (workspace Cargo, non livré) envoie le Corpus d'essai à l'interface Moteur au rythme du temps réel et écrit le texte provisoire et le texte définitif horodatés en JSONL ; Python calcule WER, latences et RTF à partir de ce JSONL. Le Banc mesure ainsi le code livré.
- Moteurs : crate officielle `sherpa-onnx` pour Nemotron et Parakeet ; `whisper-rs` (binding tiers, sur Codeberg) accepté pour comparer Whisper turbo, avec un FFI maison vers whisper.cpp en repli.
- « Toujours au premier plan » sous Wayland natif n'est garanti par aucun toolkit, quel que soit le langage ; le toolkit d'UI est Slint (ADR `0003-ui-slint`).
- Dépôt : workspace Cargo à la racine (`crates/`), `pyproject.toml` conservé pour la doc et le scoring, le paquet `src/` Python disparaît. Outillage : feature Rust du devcontainer, `cargo fmt --check` et `cargo clippy -D warnings` en pre-commit, `cargo test` et clippy en CI, Rust stable, édition 2024, runner Windows ajouté au portage.
- La diarisation future passera par ONNX (sherpa-onnx, pyannote-rs) plutôt que pyannote.audio, qui imposerait PyTorch.
- Le portage mobile reste ouvert (Rust Tier 2 sur Android et iOS), sans avoir pesé sur le choix.
