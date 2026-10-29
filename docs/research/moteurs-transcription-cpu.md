# Moteurs de transcription multi-langue temps réel sur CPU Ryzen

Recherche pour l'issue [#2](https://github.com/castorfou/transcripteur/issues/2).

**Question** : quels Moteurs locaux tiennent le temps réel sur le CPU d'un **Ryzen 5 7640U**
(6 cœurs Zen 4 / 12 threads, AVX-512, Radeon 760M intégrée, pas de CUDA) avec une bonne
qualité en **anglais, allemand et français**, et comment gèrent-ils une **Langue parlée**
qui change au cours d'une **Session** ?

**Convention** : tout chiffre marqué **[publié]** vient de la source citée.
Ce qui est marqué **[estimation]** est une extrapolation de ma part, avec son raisonnement.
Aucun benchmark n'a été lancé sur la machine cible : il faudra en faire un (voir la fin du document).

---

## 1. Ce qu'il faut savoir avant de comparer

### 1.1 Il n'existe aucun benchmark publié sur un 7640U

Le fil de benchmarks officiel de whisper.cpp ([issue #89](https://github.com/ggml-org/whisper.cpp/issues/89))
ne contient aucun Zen 4 mobile. Les points les plus proches **[publié]** (temps d'encodeur, en ms) :

| CPU | Threads | tiny | base | small | medium | large |
|---|---|---|---|---|---|---|
| Ryzen 9 5950X (Zen 3, desktop) | 8 | 197 | 421 | 1393 | 4404 | 8118 |
| Ryzen 5 4500U (Zen 2, mobile) | 6 | 672 (.en) | 1496 (.en) | 6371 (.en) | 19673 (.en) | – |
| Ryzen 7 6800H (Zen 3+, mobile) | 8 | – | 661 (.en) | – | – | – |
| Ryzen AI 9 HX 375 (Zen 5, AVX-512) | 16 | – | 437 (.en) | – | – | – |

**[estimation]** Le 7640U devrait se placer entre le 6800H et le HX 375, soit environ
1,2 à 1,6 fois plus lent que le 5950X à 8 threads. Ça donne à peu près 1,7-2,2 s
pour l'encodeur `small`, 5-7 s pour `medium` et 10-13 s pour `large`/`turbo`,
par fenêtre de 30 s.

### 1.2 Le piège de la fenêtre de 30 s de Whisper en live

Whisper encode toujours une fenêtre de 30 s : les implémentations complètent l'audio
avec du silence jusqu'à 30 s. Un affichage live qui retranscrit l'audio toutes les
500 ms (mode par défaut de `whisper-stream`, `--step 500 --length 5000`,
[README stream](https://github.com/ggml-org/whisper.cpp/blob/master/examples/stream/README.md))
paie donc **un encodage complet à chaque pas**.

- **[estimation]** Avec `large-v3`/`turbo`, environ 10 s d'encodeur par pas est
  incompatible avec ce mode.
- **[estimation]** Seuls `base` et `small` passent avec un pas de 0,5-2 s.
- La voie réaliste pour les grands modèles Whisper sur CPU est un **découpage par VAD**
  (Silero) : on transcrit chaque énoncé une seule fois quand il se termine. Les deux
  implémentations le proposent :
  - `vad_filter` / `BatchedInferencePipeline` dans [faster-whisper](https://github.com/SYSTRAN/faster-whisper) ;
  - `--vad` et `--step 0` (fenêtre glissante) dans [whisper.cpp](https://github.com/ggml-org/whisper.cpp).

  La contrepartie est une latence d'affichage d'environ 1 phrase.

Les modèles transducteurs (Parakeet, Nemotron) et les modèles nativement streaming
(Moonshine streaming, Kyutai, Voxtral Realtime, Vosk) n'ont pas cette contrainte.

---

## 2. Fiches par Moteur

### 2.1 Whisper : référence de qualité

Les WER sont tirés du papier Whisper, tables 10, 11 et 13
([arXiv 2212.04356](https://arxiv.org/abs/2212.04356)), **[publié]**,
en décodage glouton :

| Modèle | Params | FLEURS EN | FLEURS DE | FLEURS FR | CV9 DE | CV9 FR | MLS DE | MLS FR |
|---|---|---|---|---|---|---|---|---|
| tiny | 39 M | 12.4 | 27.8 | 41.4 | 34.5 | 49.7 | 24.9 | 36.8 |
| base | 74 M | 8.9 | 17.9 | 28.5 | 24.5 | 37.3 | 17.7 | 26.6 |
| small | 244 M | 6.1 | 10.2 | 15.0 | 13.0 | 22.7 | 10.5 | 16.2 |
| medium | 769 M | 4.4 | 6.5 | 8.7 | 8.5 | 16.0 | 7.4 | 8.9 |
| large-v2 | 1550 M | 4.2 | 4.5 | 8.3 | 6.4 | 13.9 | 5.5 | 7.3 |

Les autres variantes, en résumé :

- **large-v3** : 10 à 20 % d'erreurs en moins que large-v2 selon OpenAI, mais les
  chiffres par langue ne sont publiés qu'en graphique
  ([annonce](https://github.com/openai/whisper/discussions/1762)).
- **turbo** (large-v3-turbo, 809 M) : il garde l'encodeur large mais n'a que
  **4 couches de décodeur au lieu de 32**. Il est « similaire à large-v2 à travers
  les langues » **[publié]** ([discussion #2363](https://github.com/openai/whisper/discussions/2363)).
  Il n'est pas entraîné pour la traduction.
- **distil-large-v3** : **anglais uniquement**
  ([model card](https://huggingface.co/distil-whisper/distil-large-v3)), donc exclu.

La qualité utile commence à `small`. Avec `base`, le français est faible (28,5 % FLEURS).

**Licence** : MIT (code et poids, [openai/whisper](https://github.com/openai/whisper)).

**Comment Whisper gère la Langue parlée** : la langue est un token prédit en tête de
chaque fenêtre de 30 s. `detect_language` renvoie une probabilité par langue
([openai/whisper](https://github.com/openai/whisper)). La granularité est donc
**la fenêtre, ou l'énoncé si l'on découpe par VAD**. Il n'y a pas de code-switching
à l'intérieur d'une fenêtre : un seul token de langue est prédit par fenêtre.

#### faster-whisper (CTranslate2)

- **Vitesse [publié]** ([README](https://github.com/SYSTRAN/faster-whisper)) :
  benchmark CPU sur i7-12700K à 8 threads, modèle `small`, beam 5, sur environ 13 min d'audio.

  | Configuration | Temps | RAM |
  |---|---|---|
  | fp32 | 2 min 37 s | 2257 Mo |
  | int8 | 1 min 42 s | 1477 Mo |
  | int8, `batch_size=8` | 51 s | 3608 Mo |
  | whisper.cpp fp32, même machine | 2 min 05 s | 1049 Mo |

- **AMD et AVX-512 [publié]** : les binaires CTranslate2 choisissent AVX/AVX2/AVX512
  automatiquement. Ils utilisent oneDNN, pas MKL, sur les CPU non Intel
  ([hardware support](https://opennmt.net/CTranslate2/hardware_support.html)).
- **GPU** : CUDA uniquement, donc **aucune accélération possible sur la Radeon 760M**
  ([même page](https://opennmt.net/CTranslate2/hardware_support.html)).
- **Facteur temps réel [estimation]** pour le 7640U en int8, 6 threads, audio découpé par VAD.

  | Modèle | RTF estimé | Avis |
  |---|---|---|
  | `small` | ≈ 0,1-0,2 | confortable |
  | `medium` | ≈ 0,35-0,6 | tient, mais occupe le CPU |
  | `turbo` | ≈ 0,4-0,7 | grâce au décodeur à 4 couches, l'encodeur domine |
  | `large-v3` | ≈ 1 ou plus | **ne tient pas** |

- **Langue parlée [publié]**, docstrings de
  [transcribe.py](https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/transcribe.py) :
  - `multilingual=True` relance la détection **à chaque fenêtre d'encodeur** et
    change le token de langue en cours de transcription ;
  - `language_detection_threshold` vaut 0,5 et `language_detection_segments` vaut 1 ;
  - `detect_language()` renvoie toutes les probabilités.

  Le coût est faible : la détection réutilise l'encodage déjà calculé plus un pas de décodeur.
- **Bindings** : Python natif. Côté Rust, faster-whisper lui-même n'existe pas, mais
  [ct2rs](https://github.com/jkawamoto/ctranslate2-rs) (MIT) lie CTranslate2 avec la
  feature `whisper`. La logique VAD et multilingue de faster-whisper serait alors à réécrire.
- **Licence** : MIT.

#### whisper.cpp (ggml)

- **Mémoire [publié]** ([README](https://github.com/ggml-org/whisper.cpp)) :

  | Modèle | Disque | RAM |
  |---|---|---|
  | tiny | 75 Mio | ≈ 273 Mo |
  | base | 142 Mio | ≈ 388 Mo |
  | small | 466 Mio | ≈ 852 Mo |
  | medium | 1,5 Gio | ≈ 2,1 Go |
  | large | 2,9 Gio | ≈ 3,9 Go |

  La quantification entière (q5_0, q8_0…) réduit ces valeurs.
- **Backends [publié]** : AVX (CPU x86), **Vulkan**, OpenVINO, OpenBLAS, ROCm/HIP,
  VitisAI (NPU Ryzen AI) ([README](https://github.com/ggml-org/whisper.cpp)).
  Le backend NPU (VitisAI) n'est pas évalué ici : la prise en charge du NPU propre
  au 7640U reste à vérifier. Le README ne liste pas le 760M pour ROCm.
  **Vulkan via Mesa RADV est donc la voie GPU la plus réaliste.**
- **Vulkan sur iGPU RDNA3, hors sources primaires** (le 780M a 12 CU, le 760M en a 8) :
  - un utilisateur annonce un audio de 60 s transcrit en 3,5 s (RTF 0,059) sur
    Radeon 780M en Vulkan, quantification Q5_K
    ([model card tierce](https://huggingface.co/MECHUK/whisper-large-v3-turbo-russian)) ;
  - un autre annonce turbo q5 sur 780M : 10 s de dictée en ≈ 0,9 s, contre ≈ 4 s sur CPU
    ([VoiceSnip v3.5.0](https://github.com/Stefan-Schmidbauer/voicesnip/releases/tag/v3.5.0)).
  - **[estimation]** Le 760M devrait être 1,3 à 1,5 fois plus lent que le 780M :
    **turbo q5 à RTF ≈ 0,1-0,15 via Vulkan**, ce qui libère le CPU.
- **Facteur temps réel CPU [estimation]** : du même ordre que faster-whisper int8.
  whisper.cpp est un peu plus rapide en fp32 et utilise moins de RAM sur la même
  machine ([benchmark faster-whisper](https://github.com/SYSTRAN/faster-whisper)).
- **Langue parlée [publié]** ([CLI README](https://github.com/ggml-org/whisper.cpp/blob/master/examples/cli/README.md)) :
  - `-l auto` détecte la langue **à chaque appel** de transcription ;
  - `-dl` détecte la langue puis sort.

  Avec un découpage par VAD, la granularité est l'énoncé.
- **Streaming** : `whisper-stream` est qualifié d'« exemple naïf » par ses auteurs
  ([README stream](https://github.com/ggml-org/whisper.cpp/blob/master/examples/stream/README.md)).
  La logique de streaming est à écrire soi-même, ou à reprendre d'un projet comme
  [SimulStreaming](https://github.com/ufal/SimulStreaming). Ce dernier (MIT) prend en
  charge `--lan auto`, mais repose sur PyTorch et se dit lui-même « trop lent pour le
  temps réel » sur CPU.
- **Bindings [publié]** : Python avec `pywhispercpp` et d'autres, Rust avec
  **whisper-rs** ([liste du README](https://github.com/ggml-org/whisper.cpp)).
- **Licence** : MIT.

### 2.2 NVIDIA Parakeet TDT 0.6B v3 : le meilleur rapport qualité/vitesse CPU

- **Langues et licence [publié]** : 25 langues européennes, dont EN, DE et FR ;
  600 M de paramètres ; **CC-BY-4.0**, usage commercial autorisé
  ([model card](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)).
- **WER [publié]** (même source, glouton, sans ponctuation ni casse) :

  | Langue | FLEURS | MLS | CoVoST |
  |---|---|---|---|
  | EN | 4.85 | – | 6.80 |
  | DE | 5.04 | – | 4.84 |
  | FR | 5.15 | 4.97 | 6.05 |

  C'est au niveau de Whisper large-v2/v3 en DE et nettement mieux en FR,
  pour un modèle 2,5 fois plus petit.
- **Langue parlée [publié]** : le modèle « détecte automatiquement la langue de
  l'audio et transcrit sans prompt ». La model card ne dit pas si une **étiquette de
  langue** est émise, ni comment se comporte le code-switching. Pour afficher la
  Langue parlée, il faudrait donc probablement un détecteur séparé, par exemple
  l'identification de langue de sherpa-onnx basée sur Whisper. **[à vérifier]**
- **CPU via sherpa-onnx [publié]**
  ([page NeMo transducer](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/nemo-transducer-models.html)) :
  - modèle `sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8`, avec un encodeur int8 de 622 Mo ;
  - à 2 threads (CPU non précisé), RTF de 0,325 sur un clip de 3,8 s et
    **RTF de 0,028 en VAD + offline sur 334 s**.

  Pour comparer, la v2 anglaise sur ARM Cortex-A76 (RK3588) atteint un RTF de
  0,088 à 4 threads. **[estimation]** Sur le 7640U, il faut s'attendre à un **RTF
  ≈ 0,03-0,08** avec découpage VAD, soit une marge très confortable, avec environ
  1 Go de RAM.
- **Streaming** : le modèle n'est **pas streaming** au sens cache-aware. Il se
  pilote par VAD, ou par chunks via le script NeMo. parakeet-rs signale une limite
  d'environ 4-5 min par appel ([parakeet-rs](https://github.com/altunenes/parakeet-rs)).
- **Bindings [publié]** :
  - [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) (Apache-2.0) fournit
    **Python et Rust**, parmi 12 langages ;
  - [parakeet-rs](https://github.com/altunenes/parakeet-rs) (MIT/Apache-2.0)
    est en Rust pur sur ONNX Runtime, avec la v3 « 25 langues avec auto-détection »
    et un backend burn expérimental en Vulkan.

### 2.3 NVIDIA Canary (1B v2, 180M flash)

- **canary-1b-v2 [publié]** ([model card](https://huggingface.co/nvidia/canary-1b-v2)) :
  - 978 M de paramètres, 25 langues, CC-BY-4.0 ;
  - fait aussi la traduction (vers et depuis l'anglais) ;
  - les WER ne sont publiés qu'en moyenne : FLEURS 8,40 % sur 25 langues, MLS 7,27 %.
  - **La langue source doit être fournie** (`source_lang` dans tous les exemples) :
    il n'y a pas d'auto-détection documentée.
  - Il n'existe pas de build sherpa-onnx documenté.
- **canary-180m-flash dans sherpa-onnx [publié]**
  ([page Canary](https://k2-fsa.github.io/sherpa/onnx/nemo/canary.html)) :
  - couvre exactement EN, ES, DE et FR, en int8 (≈ 200 Mo) ;
  - RTF de 0,12-0,18 à 2 threads ;
  - `--canary-src-lang` est **obligatoire** (défaut `en`).
- **Langue parlée** : un détecteur externe est indispensable. La traduction intégrée
  peut intéresser le futur Enrichissement « traduction ».
- **Bindings** : Python via sherpa-onnx. Rust via sherpa-onnx en théorie, mais la page
  ne documente que le CLI et Python. **[à vérifier]**

### 2.4 NVIDIA Nemotron 3.5 ASR Streaming 0.6B : candidat récent à surveiller

- **Langues et licence [publié]** ([model card](https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b)) :
  - modèle FastConformer-RNNT **cache-aware, réellement streaming**, avec des chunks
    de 80 ms à 1,12 s ;
  - 40 locales, dont en-US, en-GB, de-DE, fr-FR et fr-CA dans le tier
    « transcription-ready » ;
  - licence **OpenMDW-1.1**, usage commercial autorisé.
- **WER FLEURS [publié]**, du chunk de 80 ms à celui de 1,12 s, en mode auto-détection :

  | Langue | 80 ms → 1,12 s |
  |---|---|
  | EN | 9,72 → 8,84 |
  | DE | 9,90 → 8,22 |
  | FR | 11,03 → 9,02 |

  C'est moins bon que Parakeet v3 offline, mais avec une vraie latence de streaming.
- **Langue parlée [publié]** : `target_lang=auto` fait émettre une **étiquette de
  langue** (`<fr-FR>`) **par énoncé**, ce qui répond directement au besoin.
  Le code-switching à l'intérieur d'un énoncé n'est pas documenté.
- **Runtimes [publié]** :
  - NeMo et Transformers ;
  - NeMo-Speech.cpp (GGUF q8_0) ;
  - [parakeet-rs](https://github.com/altunenes/parakeet-rs) en Rust, qui le liste
    comme « multilingual 3.5 0.6B, 40 language-locales ».

  Le matériel listé est uniquement NVIDIA et **aucun chiffre CPU n'est publié**.
  **[estimation]** Avec 0,6 B de paramètres comme Parakeet, le CPU devrait tenir,
  mais c'est à mesurer.

### 2.5 Vosk (Kaldi)

- **WER [publié]** ([liste des modèles](https://alphacephei.com/vosk/models)) :

  | Modèle | Taille | WER publiés |
  |---|---|---|
  | `vosk-model-en-us-0.22` | 1,8 Go | LibriSpeech 5,69, TED-LIUM 6,05 |
  | `vosk-model-de-0.21` | 1,9 Go | CV 12,82, MLS 12,42, podcast 24,00 |
  | `vosk-model-fr-0.22` | 1,4 Go | CV 14,72, MLS 11,64, podcast 21,61 |
  | petits modèles (≈ 40-50 Mo) | ≈ 40-50 Mo | FR CV 23,95, DE Tuda 13,75 |

  La plupart des modèles sont sous Apache-2.0, mais certains sont en AGPL, LGPL ou
  CC-BY-NC-SA.
- **Points forts [publié]** : streaming « zero-latency », très léger, bindings
  **Python et Rust** officiels ([vosk-api](https://github.com/alphacep/vosk-api)).
  **[estimation]** Les grands modèles tiennent largement le temps réel sur un seul cœur.
- **Langue parlée** : **un modèle par langue**, sans détection de langue
  (non mentionnée dans le README). Changer de langue implique de faire tourner
  plusieurs modèles en parallèle ou d'ajouter un LID externe.
- **Qualité** : DE et FR sont 2 à 3 fois moins bons que Parakeet ou Whisper medium.

### 2.6 Moonshine

- **Modèles [publié]** ([available models](https://moonshine-voice.readthedocs.io/en/latest/models/available-models/)) :
  - modèles streaming en MIT ;
  - EN Medium Streaming, 245 M, WER 6,65 % ;
  - **DE Small Streaming, 123 M, 7,5 %** et Tiny Streaming, 34 M, 12,0 % ;
  - **pas de français**.
- **Langue parlée** : modèle choisi par code de langue, sans auto-détection documentée.
- **Bindings [publié]** : Python, C/C++, Swift, Android, WASM, **pas de Rust**
  ([adding the library](https://moonshine-voice.readthedocs.io/en/latest/using/adding-the-library/)).
  sherpa-onnx accepte Moonshine tiny (anglais).
- **Verdict** : exclu tant que le français manque.

### 2.7 Autres candidats sérieux

| Moteur | Langues EN/DE/FR | LID | Points clés | Pourquoi pas (encore) sur ce CPU |
|---|---|---|---|---|
| **Kyutai STT 1B en_fr** ([dépôt](https://github.com/kyutai-labs/delayed-streams-modeling)) | EN, FR, **pas DE** | non documenté | streaming 0,5 s, VAD sémantique ; serveur **Rust** (candle) + Python ; code MIT/Apache, poids CC-BY-4.0 | pas d'allemand ; aucun chiffre CPU publié |
| **Voxtral Mini 4B Realtime** ([model card](https://huggingface.co/mistralai/Voxtral-Mini-4B-Realtime-2602)) | 13 langues ; FLEURS EN 4,90 / FR 6,42 / DE 6,19 à 480 ms **[publié]** | non documenté | Apache-2.0 ; vLLM ; ports C ([voxtral.c](https://github.com/antirez/voxtral.c)) et Rust communautaires | 4 B paramètres, GPU ≥ 16 Go recommandé ; sur M3 Max en BLAS CPU, l'encodeur met ≈ 8 s pour 3,6 s d'audio **[publié]** : hors de portée |
| **Qwen3-ASR 0.6B** ([model card](https://huggingface.co/Qwen/Qwen3-ASR-0.6B)) | 30 langues ; FLEURS EN 4,39 ; DE/FR non détaillés | **oui, auto** : précision 97,1 % sur FLEURS **[publié]** | Apache-2.0 ; listé dans sherpa-onnx | streaming seulement via vLLM ; environ 0,9 B paramètres ; pas de chiffre CPU |
| **Cohere Transcribe 03-2026** ([model card](https://huggingface.co/CohereLabs/cohere-transcribe-03-2026)) | 14 langues dont EN/DE/FR ; moyenne Open ASR EN 5,42 | **non** (« pas de détection automatique ») ; code-switching « inconsistant » | Apache-2.0, accès gated ; port Rust et GGUF Vulkan (transcribe.cpp) | 2 B paramètres, pas de streaming, langue à fournir |

---

## 3. Synthèse comparative

Le RTF est donné sur le CPU du 7640U, sauf mention contraire, et les chiffres sont des
**[estimations]** sauf indication contraire.

| Moteur / taille | WER FLEURS EN / DE / FR | RTF 7640U | RAM | Streaming | Langue parlée | Python | Rust | Licence |
|---|---|---|---|---|---|---|---|---|
| Whisper small (fw int8 / w.cpp) | 6,1 / 10,2 / 15,0 | ≈ 0,1-0,2 | ≈ 0,9-1,5 Go | pseudo (VAD/fenêtre) | auto, par fenêtre/énoncé, avec proba | ✅ | ✅ whisper-rs / ct2rs | MIT |
| Whisper medium | 4,4 / 6,5 / 8,7 | ≈ 0,35-0,6 | ≈ 2 Go | VAD | idem | ✅ | ✅ | MIT |
| Whisper turbo, CPU | ≈ large-v2 (4,2 / 4,5 / 8,3) | ≈ 0,4-0,7 | ≈ 2-3 Go (quantifié) | VAD | idem | ✅ | ✅ | MIT |
| Whisper turbo, **Vulkan 760M** (w.cpp) | idem | ≈ 0,1-0,15 | VRAM partagée | VAD | idem | ✅ | ✅ | MIT |
| **Parakeet TDT 0.6B v3** (sherpa-onnx int8) | **4,85 / 5,04 / 5,15** | **≈ 0,03-0,08** | ≈ 1 Go | VAD / chunks | auto implicite, **pas d'étiquette documentée** | ✅ | ✅ sherpa-onnx, parakeet-rs | CC-BY-4.0 |
| Canary 180M flash (sherpa-onnx) | n.d. par langue | ≈ 0,1-0,2 | ≈ 0,3 Go | VAD | **langue à fournir** | ✅ | ? | CC-BY-4.0 |
| **Nemotron 3.5 Streaming 0.6B** | 8,8 / 8,2 / 9,0 (chunk 1,12 s, auto) | inconnu (0,6 B) | ≈ 1 Go ? | **vrai streaming** | **auto + étiquette par énoncé** | ✅ NeMo | ✅ parakeet-rs | OpenMDW-1.1 |
| Vosk grands modèles | LibriSpeech 5,7 / CV 12,8 / CV 14,7 | ≪ 0,1 | 1,4-1,9 Go par langue | vrai streaming | **aucune** (1 modèle par langue) | ✅ | ✅ | Apache-2.0 (la plupart) |
| Moonshine streaming | 6,65 / 7,5 / **pas de FR** | faible | < 0,5 Go | vrai streaming | aucune | ✅ | ❌ | MIT |

---

## Ce que ça implique pour la décision

1. **Whisper large-v3 (non turbo) ne tient pas le temps réel sur ce CPU.** Turbo et
   medium passent seulement avec un **découpage par VAD**, pas avec une retranscription
   glissante à 500 ms. `small` est le plus grand Whisper vraiment à l'aise sur CPU,
   mais sa qualité en français (15 % FLEURS) est médiocre.
2. **Parakeet TDT 0.6B v3 est le meilleur compromis qualité/vitesse publié pour
   EN/DE/FR sur CPU.** Il offre une qualité de niveau Whisper large (meilleure en FR),
   un RTF publié de 0,028 en VAD sur 2 threads, Python et Rust via sherpa-onnx et
   parakeet-rs, et une licence permissive. Sa faiblesse : l'auto-détection de langue
   est implicite et ne fournit pas d'étiquette documentée. Afficher la Langue parlée
   demandera donc probablement un **LID séparé** (Whisper tiny/base en détection seule,
   par exemple).
3. **Whisper reste la référence pour la Langue parlée.** Sa détection explicite avec
   probabilités par langue est réévaluée par fenêtre (`multilingual=True` dans
   faster-whisper, `-l auto` par appel dans whisper.cpp). Le **backend Vulkan de
   whisper.cpp** est le seul moyen d'utiliser la Radeon 760M. **[estimation]**
   Il rendrait turbo confortable (RTF ≈ 0,1-0,15), à valider sur la machine.
4. **Nemotron 3.5 Streaming est le candidat qui coche le plus de cases sur le
   papier** : vrai streaming, étiquette de langue par énoncé, EN/DE/FR, Rust via
   parakeet-rs. Mais il n'a **aucun chiffre CPU publié**, sa licence (OpenMDW) est
   moins connue et sa qualité est inférieure à Parakeet v3 offline.
5. **Le code-switching à l'intérieur d'un énoncé n'est bien géré par aucun candidat.**
   Tous travaillent au mieux par fenêtre ou par énoncé. Ça colle au modèle du domaine,
   où la Langue parlée est définie « pour un passage d'audio ».
6. **Vosk, Moonshine, Kyutai, Canary et Cohere** sont écartés comme Moteur principal,
   chacun pour une raison bloquante :
   - Vosk : qualité DE/FR et aucune détection de langue ;
   - Moonshine : pas de français ;
   - Kyutai : pas d'allemand ;
   - Canary et Cohere : langue à fournir.

   Voxtral Realtime est hors de portée du CPU.

**Recommandation courte** : prototyper deux Moteurs derrière la même interface de Plugin.

- **(a)** Parakeet TDT v3 int8 via sherpa-onnx, découpé par VAD, avec un LID Whisper léger.
- **(b)** Whisper turbo via whisper.cpp, en CPU puis en Vulkan, avec `-l auto`.

Mesurer le RTF réel et la latence d'affichage sur le 7640U, puis tester Nemotron 3.5
Streaming dès qu'un runtime CPU (parakeet-rs ou NeMo-Speech.cpp) est disponible.
sherpa-onnx et whisper.cpp ont tous deux des bindings Python et Rust : ce choix de
Moteur ne contraint pas le choix du langage.

### Mesures à faire sur la machine cible

- RTF de Parakeet v3 int8 avec sherpa-onnx à 4, 6 et 12 threads.
- RTF de Whisper small, medium et turbo en int8/q5 avec whisper.cpp, sur CPU et en Vulkan (RADV).
- Coût réel d'un LID Whisper par énoncé.
- Comportement de Parakeet v3 sur un enregistrement qui alterne FR, DE et EN.
