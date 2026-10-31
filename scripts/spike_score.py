"""PROTOTYPE : score les JSONL du Spike de bout en bout (WER, latences, RTF, langue).

Usage : python scripts/spike_score.py data/raw/spike/<config>--<extrait>.jsonl ...

Latence du Texte provisoire : instant d'affichage d'un mot moins sa fin dans l'audio.
Les sous-titres sont trop grossiers pour dater un mot, on prend donc deux horloges :
- `auto` : le timestamp que le Moteur live donne lui-même à son dernier token ;
- `pk` : la fin du même mot selon Parakeet, Moteur indépendant, après alignement.
Ces latences n'ont de sens qu'au rythme du temps réel (`--vitesse 1`).
"""

import json
import os
import statistics
import sys
from difflib import SequenceMatcher
from pathlib import Path


sys.path.insert(0, "scripts")
from corpus import normalize_for_wer


HESITATIONS = {"uh", "um", "uhm", "hm", "hmm", "mhm", "äh", "ähm", "öh", "euh", "heu"}
REFS = Path(os.environ.get("SPIKE_REFS", "data/processed/corpus"))


def normaliser(texte: str) -> list[str]:
    """`normalize_for_wer` sans les hésitations, que les sous-titres ne notent pas."""
    return [m for m in normalize_for_wer(texte).split() if m not in HESITATIONS]


def erreurs(ref: list[str], hyp: list[str]) -> int:
    """Distance d'édition en mots."""
    avant = list(range(len(hyp) + 1))
    for i, r in enumerate(ref, 1):
        ligne = [i]
        for j, h in enumerate(hyp, 1):
            ligne.append(min(avant[j] + 1, ligne[-1] + 1, avant[j - 1] + (r != h)))
        avant = ligne
    return avant[-1]


def wer(ref: str, hyp: str) -> float:
    r, h = normaliser(ref), normaliser(hyp)
    return erreurs(r, h) / max(len(r), 1)


def centile(valeurs: list[float], q: float) -> float:
    return (
        sorted(valeurs)[min(int(q * len(valeurs)), len(valeurs) - 1)]
        if valeurs
        else float("nan")
    )


def mots_live(ev: list[dict]) -> list[tuple[str, float, float]]:
    """(mot, audio de son dernier token, instant d'affichage) du Moteur live."""
    mots, courant = [], None
    for e in ev:
        if e["ev"] != "provisoire":
            continue
        for tok, ts in e["nouveaux"]:
            if tok.startswith(" ") or courant is None:
                if courant:
                    mots.append(tuple(courant))
                courant = [tok.strip(), ts, e["t"]]
            else:
                courant[0] += tok
                courant[1:] = [ts, e["t"]]
    return mots + ([tuple(courant)] if courant else [])


def mots_parakeet(ev: list[dict]) -> list[tuple[str, float]]:
    """(mot, fin dans l'audio) selon Parakeet."""
    mots = []
    for e in ev:
        if e["ev"] != "definitif":
            continue
        for tok, debut, duree in e["mots"]:
            fin = e["debut"] + debut + duree
            if tok.startswith(" ") or not mots:
                mots.append([tok.strip(), fin])
            else:
                mots[-1][0] += tok
                mots[-1][1] = fin
    return [tuple(m) for m in mots]


def latences_provisoire(ev: list[dict]) -> tuple[list[float], list[float]]:
    live = [(normalize_for_wer(m), a, t) for m, a, t in mots_live(ev)]
    live = [m for m in live if m[0]]
    pk = [(normalize_for_wer(m), f) for m, f in mots_parakeet(ev)]
    pk = [m for m in pk if m[0]]
    auto = [t - a for _, a, t in live]
    sm = SequenceMatcher(None, [m[0] for m in live], [m[0] for m in pk], autojunk=False)
    croise = [
        live[b.a + k][2] - pk[b.b + k][1]
        for b in sm.get_matching_blocks()
        for k in range(b.size)
    ]
    return auto, croise


def segments_multilingue() -> list[tuple[float, float, str]]:
    """(début, fin, langue) des coupes réelles du montage, écrites par le recalage."""
    return json.loads((REFS / "multilingue-segments.json").read_text())


def lire_srt(chemin: Path) -> list[tuple[float, str]]:
    def secondes(h: str) -> float:
        a, b, c = h.replace(",", ".").split(":")
        return int(a) * 3600 + int(b) * 60 + float(c)

    cues = []
    for bloc in chemin.read_text().strip().split("\n\n"):
        lignes = bloc.splitlines()
        cues.append((secondes(lignes[1].split(" --> ")[0]), " ".join(lignes[2:])))
    return cues


def score_multilingue(ev: list[dict]) -> dict:
    """WER par segment : le texte suit-il la langue dès l'énoncé suivant ?"""
    cues = lire_srt(REFS / "multilingue.srt")
    par_langue: dict[str, list[int]] = {}
    etiquettes = [0, 0]
    premiers = []
    for debut, fin, langue in segments_multilingue():
        dedans = lambda x: debut <= x < fin + 0.5  # noqa: E731, B023
        defs = [e for e in ev if e["ev"] == "definitif" and dedans(e["debut"])]
        ref = " ".join(t for d, t in cues if dedans(d))
        hyp = " ".join(e["texte"] for e in defs)
        r, h = normaliser(ref), normaliser(hyp)
        cumul = par_langue.setdefault(langue, [0, 0])
        cumul[0] += erreurs(r, h)
        cumul[1] += len(r)
        etiquettes[0] += sum(e["langue"] == langue for e in defs)
        etiquettes[1] += len(defs)
        if defs:
            premiers.append(defs[0]["langue"] == langue)
    return {
        "wer_par_langue": {k: round(e / n, 3) for k, (e, n) in par_langue.items()},
        "etiquette_ok": f"{etiquettes[0]}/{etiquettes[1]}",
        "premier_enonce_ok": f"{sum(premiers)}/{len(premiers)}",
    }


def score(chemin: Path) -> dict:
    ev = [json.loads(ligne) for ligne in chemin.open()]
    config, _, extrait = chemin.stem.rpartition("--")
    bilan = next(e for e in ev if e["ev"] == "bilan")
    duree = bilan["duree"]
    defs = [e for e in ev if e["ev"] == "definitif"]
    a_defs = any(e["texte"] for e in defs)
    ligne = {
        "config": config,
        "extrait": extrait,
        "rtf_live": round(bilan["moteur"] / duree, 3),
        "rtf_2e": round(sum(e["t_calcul"] for e in defs) / duree, 3),
        "pic_mo": bilan["pic"],
    }
    ref = REFS / f"{extrait}.txt"
    if ref.exists():
        texte = ref.read_text()
        live = " ".join(e["texte"] for e in ev if e["ev"] == "fin_live")
        ligne["wer_live"] = round(wer(texte, live), 3)
        if a_defs:
            ligne["wer_2e"] = round(wer(texte, " ".join(e["texte"] for e in defs)), 3)
    args = next(e for e in ev if e["ev"] == "config")["args"]
    if args.get("vitesse") == "1":
        auto, croise = latences_provisoire(ev)
        ligne["prov_auto_p50_p95"] = (
            round(centile(auto, 0.5), 2),
            round(centile(auto, 0.95), 2),
        )
        if croise:
            ligne["prov_pk_p50_p95"] = (
                round(centile(croise, 0.5), 2),
                round(centile(croise, 0.95), 2),
            )
        if a_defs:
            lat = [e["t"] - e["fin"] for e in defs]
            ligne["def_p50_p95_max"] = (
                round(statistics.median(lat), 2),
                round(centile(lat, 0.95), 2),
                round(max(lat), 2),
            )
        rss = [e["rss"] for e in ev if e["ev"] == "sys"]
        ligne["rss_debut_fin"] = (rss[1] if len(rss) > 1 else None, rss[-1])
    if extrait == "multilingue" and a_defs:
        ligne |= score_multilingue(ev)
    if defs:
        langues: dict[str, int] = {}
        for e in defs:
            langues[e["langue"]] = langues.get(e["langue"], 0) + 1
        ligne["langues"] = langues
    return ligne


if __name__ == "__main__":
    for chemin in sys.argv[1:]:
        print(json.dumps(score(Path(chemin)), ensure_ascii=False))
