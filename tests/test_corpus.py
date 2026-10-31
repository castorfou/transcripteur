"""Tests du découpage et de la normalisation du Corpus d'essai."""

import pytest

from corpus import (
    Cue,
    build_srt,
    clean_cue_text,
    normalize_for_wer,
    parse_srt,
    select_cues,
)


SRT = """1
00:00:01,000 --> 00:00:03,500
Bonjour à tous,
et merci.

2
00:00:03,500 --> 00:00:05,000
(Rires)

3
00:00:05,000 --> 00:00:07,250
AH : Alors, on commence.
"""


def test_parse_srt_reads_times_and_joins_lines() -> None:
    cues = parse_srt(SRT)
    assert cues[0] == Cue(1.0, 3.5, "Bonjour à tous,\net merci.")
    assert cues[2].end == pytest.approx(7.25)
    assert len(cues) == 3


def test_parse_srt_handles_bom_and_crlf() -> None:
    assert parse_srt("﻿1\r\n00:00:00,000 --> 00:00:01,000\r\nHallo\r\n") == [
        Cue(0.0, 1.0, "Hallo")
    ]


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("(Rires)", ""),
        ("[Applaus]", ""),
        ("*Applaus*", ""),
        ("Herald: Danke schön", "Danke schön"),
        ("Alors (Rires) on commence", "Alors on commence"),
        ("AH : Alors il faut", "Alors il faut"),
        ("Public : Oui.", "Oui."),
        ("<i>Bonjour</i>", "Bonjour"),
        ("♪ musique ♪", "musique"),
        ("Bonjour à tous,\net merci.", "Bonjour à tous, et merci."),
        ("Une phrase : avec deux-points", "Une phrase : avec deux-points"),
    ],
)
def test_clean_cue_text_removes_what_was_not_spoken(raw: str, expected: str) -> None:
    assert clean_cue_text(raw) == expected


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("Bonjour, à tous !", "bonjour à tous"),
        ("l’orthographe d'hier", "l'orthographe d'hier"),
        ("Est-ce que c'est clair ?", "est ce que c'est clair"),
        ("Die Straße, 42 Mal.", "die strasse 42 mal"),
        ("  plusieurs   espaces ", "plusieurs espaces"),
        ("« citation » – fin…", "citation fin"),
    ],
)
def test_normalize_for_wer(raw: str, expected: str) -> None:
    assert normalize_for_wer(raw) == expected


def test_select_cues_keeps_only_cues_fully_inside_the_window() -> None:
    cues = [Cue(0, 2, "a"), Cue(2, 4, "b"), Cue(4, 6, "c"), Cue(6, 8, "d")]
    assert [c.text for c in select_cues(cues, 1, 7)] == ["b", "c"]


def test_select_cues_drops_cues_left_empty_by_cleaning() -> None:
    cues = [Cue(0, 1, "a"), Cue(1, 2, "(Rires)"), Cue(2, 3, "b")]
    assert [c.text for c in select_cues(cues, 0, 3)] == ["a", "b"]


def test_build_srt_round_trips() -> None:
    cues = [Cue(0.0, 1.5, "a"), Cue(61.25, 62.0, "b")]
    assert parse_srt(build_srt(cues)) == cues
