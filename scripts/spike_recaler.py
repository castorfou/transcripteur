"""PROTOTYPE : recale les références CCC sur l'audio, sans toucher au Corpus d'essai.

Les sous-titres de certains talks sont décalés par rapport au MP3. Pour chaque source,
on cherche le décalage qui minimise le WER contre un run du Moteur, puis on réécrit
les références dans `data/processed/corpus-recale/` (même format que le corpus).
"""

import json
import sys
import tomllib
from pathlib import Path


sys.path.insert(0, "scripts")
from dataclasses import replace

from spike_score import erreurs

from corpus import (
    build_srt,
    normalize_for_wer,
    parse_srt,
    select_cues,
)


SOURCES = Path("data/raw/corpus/sources")
SORTIE = Path("data/processed/corpus-recale")
RUN = "data/raw/spike/wer-n320-pk--{}.jsonl"


def hypothese(extrait: str) -> list[str]:
    lignes = Path(RUN.format(extrait)).read_text().splitlines()
    ev = [json.loads(ligne) for ligne in lignes]
    return normalize_for_wer(
        " ".join(e["texte"] for e in ev if e["ev"] == "definitif")
    ).split()


def texte(cues: list, debut: float, fin: float) -> list[str]:
    return normalize_for_wer(
        " ".join(c.text for c in select_cues(cues, debut, fin))
    ).split()


def fenetre_audio(cues: list, s: dict) -> tuple[float, float]:
    """Bornes réelles de la coupe audio : `build_extract` coupe sur les sous-titres."""
    choisis = select_cues(cues, s["start"], s["end"])
    return choisis[0].start, choisis[-1].end


def main() -> None:
    manifeste = tomllib.loads(Path("corpus/manifest.toml").read_text())
    decalages = {}
    cues_par_source = {}
    SORTIE.mkdir(exist_ok=True)
    for e in manifeste["extract"]:
        if e["kind"] != "talk":
            continue
        chemin = SOURCES / f"{e['source']}.reference.srt"
        cues = parse_srt(chemin.read_text())
        cues_par_source[e["source"]] = cues
        hyp = hypothese(e["id"])
        debut, fin = fenetre_audio(cues, e)
        scores = []
        # Décalage d : l'audio à l'instant t correspond au sous-titre à t + d.
        for pas in range(-10, 61):
            d = pas / 2
            r = texte(cues, debut + d, fin + d)
            scores.append((erreurs(r, hyp) / max(len(r), 1), d, len(r)))
        wer, d, n = min(scores)
        sans = next(s for s in scores if s[1] == 0)
        print(
            f"{e['id']}: décalage {d:+.1f} s, WER {sans[0]:.3f} -> {wer:.3f} ({n} mots)"
        )
        decalages[e["source"]] = d

    for e in manifeste["extract"]:
        if e["kind"] not in ("talk", "multilingue"):
            continue
        segments = e.get("segments") or [e]
        recales, pos = [], 0.0
        coupes = []
        for s in segments:
            cues = cues_par_source[s["source"]]
            d = decalages[s["source"]]
            debut, fin = fenetre_audio(cues, s)
            for c in select_cues(cues, debut + d, fin + d):
                recales.append(
                    replace(
                        c, start=c.start - d - debut + pos, end=c.end - d - debut + pos
                    )
                )
            coupes.append((pos, pos + fin - debut, s["source"].split("-")[0]))
            pos += fin - debut + 1
        (SORTIE / f"{e['id']}.srt").write_text(build_srt(recales))
        (SORTIE / f"{e['id']}.txt").write_text(
            normalize_for_wer(" ".join(c.text for c in recales)) + "\n"
        )
        if e["id"] == "multilingue":
            (SORTIE / "multilingue-segments.json").write_text(json.dumps(coupes))
    (SORTIE / "decalages.json").write_text(json.dumps(decalages, indent=1))


if __name__ == "__main__":
    main()
