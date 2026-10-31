"""Recrée le Corpus d'essai à partir de `corpus/manifest.toml`.

Usage : uv run --active --extra corpus python scripts/corpus.py [--only ID ...]

L'audio va dans `data/raw/corpus/`, les références dans `data/processed/corpus/`.
Ni l'un ni l'autre n'est versionné : le manifeste fige les URL et les empreintes.
"""

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import tomllib
import unicodedata
import urllib.request
import wave
from dataclasses import dataclass, replace
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.toml"
AUDIO_DIR = ROOT / "data" / "raw" / "corpus"
SOURCES_DIR = AUDIO_DIR / "sources"
REFERENCE_DIR = ROOT / "data" / "processed" / "corpus"

SAMPLE_RATE = 16_000
SILENCE_BETWEEN_SEGMENTS_S = 1.0

TIMECODE = r"(\d+):(\d\d):(\d\d)[,.](\d{3})"
CUE_TIMES = re.compile(rf"{TIMECODE}\s*-->\s*{TIMECODE}")
NOT_SPOKEN = re.compile(r"<[^>]+>|\([^)]*\)|\[[^\]]*\]|\*[^*]*\*|♪")
SPEAKER_LABEL = re.compile(r"^(?:[A-ZÉ]{1,3}|Public|Herald|Publikum)\s*:\s*")
OUTER_APOSTROPHE = re.compile(r"(?<!\w)'|'(?!\w)")


@dataclass(frozen=True)
class Cue:
    """Un sous-titre : texte affiché entre deux instants, en secondes."""

    start: float
    end: float
    text: str


def _seconds(hours: str, minutes: str, seconds: str, millis: str) -> float:
    return int(hours) * 3600 + int(minutes) * 60 + int(seconds) + int(millis) / 1000


def parse_srt(content: str) -> list[Cue]:
    """Lit un fichier SRT."""
    cues = []
    for block in re.split(r"\n\s*\n", content.lstrip("﻿").replace("\r\n", "\n")):
        lines = block.strip().split("\n")
        for index, line in enumerate(lines):
            times = CUE_TIMES.search(line)
            if times:
                groups = times.groups()
                cues.append(
                    Cue(
                        _seconds(*groups[:4]),
                        _seconds(*groups[4:]),
                        "\n".join(lines[index + 1 :]),
                    )
                )
                break
    return cues


def _timecode(seconds: float) -> str:
    total_millis = round(seconds * 1000)
    hours, rest = divmod(total_millis, 3_600_000)
    minutes, rest = divmod(rest, 60_000)
    secs, millis = divmod(rest, 1000)
    return f"{hours:02}:{minutes:02}:{secs:02},{millis:03}"


def build_srt(cues: list[Cue]) -> str:
    """Écrit des sous-titres au format SRT."""
    return "\n".join(
        f"{i}\n{_timecode(c.start)} --> {_timecode(c.end)}\n{c.text}\n"
        for i, c in enumerate(cues, 1)
    )


def clean_cue_text(raw: str) -> str:
    """Retire d'un sous-titre ce qui n'a pas été prononcé.

    Annotations (rires, applaudissements), balises et noms d'orateurs partent ;
    la ponctuation reste, elle sert à la lecture humaine.
    """
    text = " ".join(NOT_SPOKEN.sub(" ", raw).split())
    return SPEAKER_LABEL.sub("", text)


def normalize_for_wer(text: str) -> str:
    """Normalise un texte pour le WER ; à appliquer aussi au texte du Moteur."""
    text = unicodedata.normalize("NFKC", text).casefold().replace("’", "'")
    text = re.sub(r"[^\w\s']", " ", text)
    return " ".join(OUTER_APOSTROPHE.sub(" ", text).split())


def select_cues(cues: list[Cue], start: float, end: float) -> list[Cue]:
    """Garde les sous-titres entièrement dans la fenêtre, nettoyés et non vides."""
    selected = []
    for cue in cues:
        if cue.start >= start and cue.end <= end:
            text = clean_cue_text(cue.text)
            if text:
                selected.append(replace(cue, text=text))
    return selected


def _download(url: str, destination: Path) -> None:
    partial = destination.with_suffix(destination.suffix + ".part")
    with urllib.request.urlopen(url, timeout=60) as response, partial.open("wb") as out:
        shutil.copyfileobj(response, out)
    partial.rename(destination)


def _yt_dlp(*args: str) -> None:
    subprocess.run([sys.executable, "-m", "yt_dlp", "--quiet", *args], check=True)


def fetch_audio(source: dict) -> Path:
    """Télécharge l'audio complet d'une source, une seule fois."""
    existing = list(SOURCES_DIR.glob(f"{source['id']}.audio.*"))
    if existing:
        return existing[0]
    audio = source["audio"]
    if audio.startswith("youtube:"):
        _yt_dlp(
            "-f",
            "ba[format_id!*=drc]",
            "-o",
            str(SOURCES_DIR / f"{source['id']}.audio.%(ext)s"),
            f"https://www.youtube.com/watch?v={audio.removeprefix('youtube:')}",
        )
        return next(SOURCES_DIR.glob(f"{source['id']}.audio.*"))
    destination = SOURCES_DIR / f"{source['id']}.audio.mp3"
    _download(audio, destination)
    return destination


def fetch_reference(source: dict) -> list[Cue]:
    """Télécharge les sous-titres d'une source et vérifie leur empreinte."""
    destination = SOURCES_DIR / f"{source['id']}.reference.srt"
    if not destination.exists():
        reference = source["reference"]
        if reference.startswith("youtube:"):
            video_id, lang = reference.removeprefix("youtube:").split("/")
            _yt_dlp(
                "--skip-download",
                "--write-subs",
                "--sub-langs",
                lang,
                "--sub-format",
                "srt",
                "-o",
                str(SOURCES_DIR / f"{source['id']}.reference"),
                f"https://www.youtube.com/watch?v={video_id}",
            )
            (SOURCES_DIR / f"{source['id']}.reference.{lang}.srt").rename(destination)
        else:
            _download(reference, destination)
    digest = hashlib.sha256(destination.read_bytes()).hexdigest()
    expected = source.get("reference_sha256")
    if not expected:
        print(f'{source["id"]} : reference_sha256 = "{digest}"')
    elif digest != expected:
        raise SystemExit(
            f"{source['id']} : la référence a changé à la source "
            f"(attendu {expected}, obtenu {digest}). Les mesures ne seraient "
            "plus comparables : vérifier le fichier puis mettre à jour le manifeste."
        )
    return parse_srt(destination.read_text(encoding="utf-8"))


def _cut(audio: Path, start: float, end: float, destination: Path) -> None:
    import imageio_ffmpeg  # extra `corpus`, absent de la CI

    subprocess.run(
        [
            imageio_ffmpeg.get_ffmpeg_exe(),
            "-loglevel",
            "error",
            "-y",
            "-ss",
            f"{start:.3f}",
            "-to",
            f"{end:.3f}",
            "-i",
            str(audio),
            "-ac",
            "1",
            "-ar",
            str(SAMPLE_RATE),
            "-c:a",
            "pcm_s16le",
            str(destination),
        ],
        check=True,
    )


def _concatenate(parts: list[Path], destination: Path) -> None:
    silence = b"\0\0" * int(SAMPLE_RATE * SILENCE_BETWEEN_SEGMENTS_S)
    with wave.open(str(destination), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(SAMPLE_RATE)
        for index, part in enumerate(parts):
            if index:
                out.writeframes(silence)
            with wave.open(str(part), "rb") as segment:
                out.writeframes(segment.readframes(segment.getnframes()))


def _duration(path: Path) -> float:
    with wave.open(str(path), "rb") as audio:
        return audio.getnframes() / audio.getframerate()


def build_extract(extract: dict, sources: dict[str, dict]) -> None:
    """Crée l'audio et la référence d'un extrait, à partir d'un ou plusieurs segments."""
    wav = AUDIO_DIR / f"{extract['id']}.wav"
    parts, shifted, offset = [], [], 0.0
    for index, segment in enumerate(extract.get("segments", [extract])):
        source = sources[segment["source"]]
        start, end = segment["start"], segment["end"]
        cues = []
        if extract.get("reference", True):
            cues = select_cues(fetch_reference(source), start, end)
            start, end = cues[0].start, cues[-1].end
        part = AUDIO_DIR / f"{extract['id']}.part{index}.wav"
        if not wav.exists():
            _cut(fetch_audio(source), start, end, part)
            parts.append(part)
        shifted += [
            replace(c, start=c.start - start + offset, end=c.end - start + offset)
            for c in cues
        ]
        offset += end - start + SILENCE_BETWEEN_SEGMENTS_S
    if parts:
        _concatenate(parts, wav)
        for part in parts:
            part.unlink()
    if shifted:
        (REFERENCE_DIR / f"{extract['id']}.srt").write_text(
            build_srt(shifted), encoding="utf-8"
        )
        (REFERENCE_DIR / f"{extract['id']}.txt").write_text(
            normalize_for_wer(" ".join(c.text for c in shifted)) + "\n",
            encoding="utf-8",
        )
    print(f"{extract['id']} : {_duration(wav):.0f} s, {len(shifted)} sous-titres")


def check_micro(extract: dict) -> bool:
    """Signale ce qui manque à un extrait enregistré à la main."""
    wav = AUDIO_DIR / f"{extract['id']}.wav"
    reference = REFERENCE_DIR / f"{extract['id']}.txt"
    missing = [str(p.relative_to(ROOT)) for p in (wav, reference) if not p.exists()]
    if missing:
        print(f"{extract['id']} : à fournir à la main : {', '.join(missing)}")
    return not missing


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--only", nargs="*", help="identifiants d'extraits à recréer")
    args = parser.parse_args()

    manifest = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    sources = {source["id"]: source for source in manifest["source"]}
    SOURCES_DIR.mkdir(parents=True, exist_ok=True)
    REFERENCE_DIR.mkdir(parents=True, exist_ok=True)
    for extract in manifest["extract"]:
        if args.only and extract["id"] not in args.only:
            continue
        if extract["kind"] == "micro":
            check_micro(extract)
        else:
            build_extract(extract, sources)


if __name__ == "__main__":
    main()
