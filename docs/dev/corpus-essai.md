# Corpus d'essai

Le Corpus d'essai sert à juger la qualité et la latence de l'outil. Le Banc d'essai l'envoie au Moteur au rythme du temps réel.

## Organisation

| Élément | Emplacement | Versionné |
| --- | --- | --- |
| Manifeste : URL, début et fin, langue, provenance, licence, empreintes | `corpus/manifest.toml` | oui |
| Script de construction | `scripts/corpus.py` | oui |
| Audio des extraits, WAV 16 kHz mono | `data/raw/corpus/<id>.wav` | non |
| Fichiers source téléchargés | `data/raw/corpus/sources/` | non |
| Référence normalisée pour le WER | `data/processed/corpus/<id>.txt` | non |
| Référence horodatée lisible, recalée sur l'extrait | `data/processed/corpus/<id>.srt` | non |

Les références ne sont pas versionnées : les pages sources n'indiquent pas de licence pour les sous-titres. Le manifeste fige leur empreinte SHA-256, et la construction s'arrête si une référence change à la source, car les mesures ne seraient plus comparables.

## Recréer le corpus

```bash
uv run --active --extra corpus python scripts/corpus.py
```

Le script ne recrée que ce qui manque. L'option `--only` limite la construction à certains extraits :

```bash
uv run --active --extra corpus python scripts/corpus.py --only de-klima multilingue
```

L'ensemble occupe environ 600 Mo. `yt-dlp` fonctionne sans runtime JavaScript et affiche à ce sujet des avertissements sans conséquence.

## Contenu

- **Extraits par langue** : 3 extraits d'environ 5 min en EN, DE et FR. Chaque extrait commence et finit sur un sous-titre.
- **`multilingue`** : montage d'environ 9 min de 12 segments d'environ 50 s, tirés des mêmes talks hors de leurs extraits et séparés par 1 s de silence. La langue change entre deux énoncés, jamais au milieu. Chaque changement de langue est aussi un changement de locuteur et de salle.
- **`tenue-1h`** : talk d'environ 1 h sans référence, pour la tenue de Session.
- **`micro-1` et `micro-2`** : enregistrements en salle, à fournir à la main.

## Sources

- **EN et DE** : talks de media.ccc.de, sous-titrés par la communauté c3subtitles. Seuls les fichiers relus sont retenus ; le préfixe `DRAFT_` de leur nom signale les transcriptions non relues.
- **FR** : TEDx sur YouTube, avec une piste de sous-titres français manuelle. Les pistes automatiques sont exclues.

## Normalisation

`clean_cue_text` retire ce qui n'a pas été prononcé : annotations, balises et noms d'orateurs. `normalize_for_wer` met en minuscules et retire la ponctuation. Le scoring applique `normalize_for_wer` au texte du Moteur. Les nombres restent tels quels, si bien qu'un Moteur qui écrit `quarante-deux` là où la référence porte `42` compte une erreur.

Les horodatages des sous-titres ne donnent pas l'instant de chaque mot : un sous-titre CCC dure plusieurs secondes. Ils ne suffisent pas à mesurer la latence au mot près.

## Ajouter un enregistrement micro

Les enregistrements personnels ne sont jamais versionnés.

1. Lister les Sources : `pactl list short sources`.
2. Enregistrer 5 min dans une salle :

    ```bash
    parecord --device=<source> --rate=16000 --channels=1 --file-format=wav \
      data/raw/corpus/micro-1.wav
    ```

3. Écrire la transcription corrigée à la main dans `data/processed/corpus/micro-1.brut.txt`.
4. La normaliser :

    ```bash
    uv run --active python -c "
    import sys; sys.path.insert(0, 'scripts')
    from corpus import normalize_for_wer
    from pathlib import Path
    texte = Path('data/processed/corpus/micro-1.brut.txt').read_text()
    Path('data/processed/corpus/micro-1.txt').write_text(normalize_for_wer(texte))
    "
    ```

Le script signale les enregistrements manquants.
