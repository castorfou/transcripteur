# Streaming : live provisoire puis corrigé

Question de recherche ([issue #3](https://github.com/castorfou/transcripteur/issues/3)) : quelles techniques permettent un affichage live où apparaît d'abord un **texte provisoire très rapide (< 1 s)**, ensuite **corrigé en texte définitif**, avec des Moteurs locaux ? Quelles latences obtient-on en pratique ?

Cadre : MVP desktop Linux, machine de référence Ryzen 5 7640U, **CPU seul**. Langues parlées visées : EN, DE, FR, qui peuvent alterner au cours d'une même Session.

!!! note "Convention"
    **[publié]** signale un chiffre tiré tel quel d'une source primaire, avec son matériel. **[estimation]** signale un raisonnement de l'auteur, non mesuré. Presque tous les chiffres publiés viennent de GPU ou de Mac Apple Silicon, et aucun n'a été mesuré sur la machine de référence.

## 1. Trois familles de techniques

| Famille | Principe | Provisoire → définitif |
|---|---|---|
| **A. VAD + Moteur hors ligne** | Un VAD découpe l'audio en énoncés ; chaque énoncé est transcrit en entier une fois le silence détecté. | Pas de provisoire natif. On peut en simuler un en re-transcrivant l'énoncé en cours toutes les N ms. |
| **B. Moteur hors ligne + politique de streaming** (LocalAgreement, AlignAtt) | On re-décode régulièrement un buffer audio qui grandit ; une politique décide quelle partie du texte est stable. | Oui : la partie confirmée devient définitive, la queue reste provisoire. |
| **C. Moteur nativement streaming** (transducteurs cache-aware, delayed streams, encodeurs à fenêtre glissante) | Le Moteur traite chaque trame audio une seule fois et émet des tokens en continu, avec une latence fixée. | Le texte émis est déjà quasi définitif. Une correction éventuelle vient d'une seconde passe (famille A) en fin d'énoncé. |

## 2. Segmentation par VAD

- **Silero VAD** : un chunk de 30 ms ou plus prend **moins de 1 ms sur un seul thread CPU** [publié] ; modèle d'environ 2 Mo, 8 et 16 kHz, export ONNX, exemples Rust, Go, C++ et C# dans le dépôt ([snakers4/silero-vad](https://github.com/snakers4/silero-vad)). La page annonce une licence MIT, mais un badge affiche « CC BY-NC 4.0 » : à vérifier dans le fichier LICENSE avant de l'intégrer.
- **WebRTC VAD** (`py-webrtcvad`) : trames de 10, 20 ou 30 ms, PCM 16 bits à 8, 16, 32 ou 48 kHz, quatre niveaux d'agressivité de 0 à 3 ([wiseman/py-webrtcvad](https://github.com/wiseman/py-webrtcvad)). Plus léger encore, mais c'est un modèle GMM ancien, moins robuste au bruit (qualité non chiffrée ici).
- **Rôle dans les trois familles** : en A, le VAD *est* la segmentation. En B, il sert de « voice activity controller » qui remet le buffer à zéro entre deux énoncés : `--vac` est recommandé dans [whisper_streaming](https://github.com/ufal/whisper_streaming), et VAD comme VAC sont activés par défaut dans [WhisperLiveKit](https://github.com/QuentinFuxa/WhisperLiveKit). En C, il sert surtout à couper les silences et à décider où relancer une seconde passe.
- **Effet mesuré dans whisper_streaming** : sur de l'anglais lu, désactiver le filtre VAD réduit la latence de 0,23 à 0,41 s pour un WER quasi identique. Sur de l'interprétation DE/CS riche en pauses, l'activer améliore le WER d'environ 3 points aux petits chunks [publié, GPU A40] ([Macháček et al. 2023](https://arxiv.org/html/2307.14743)).
- **Limite de la famille A seule** : le texte n'apparaît qu'après la fin de l'énoncé, donc après durée de l'énoncé + temps de silence + temps de décodage. Ça dépasse très vite 1 s. whisper.cpp le montre bien avec son exemple `stream` : soit il re-transcrit toutes les 500 ms une fenêtre glissante de 5 s, réécrite sur place (`--step 500 --length 5000`), soit il attend un silence (`--step 0` avec VAD) pour imprimer un bloc horodaté ([whisper.cpp/examples/stream](https://github.com/ggml-org/whisper.cpp/tree/master/examples/stream)).

## 3. Politiques de streaming sur Moteur hors ligne (Whisper)

### LocalAgreement (whisper_streaming)

- **Principe** : on émet comme définitif le plus long préfixe commun aux sorties de *n* re-décodages consécutifs du buffer, avec *n* = 2 (LocalAgreement-2). Le buffer est limité à environ 30 s et rogné aux fins de segments ou de phrases confirmées ([Macháček et al. 2023](https://arxiv.org/html/2307.14743)).
- **Latences** [publié, GPU NVIDIA A40, Whisper large-v2, `MinChunkSize` = 1 s] : **3,3 s (EN), 4,4 s (DE), 4,8 s (CS)** en moyenne, avec un écart-type de 0,36 à 0,81 s entre exécutions. La latence vaut environ deux fois la taille de chunk, et le WER est environ 2 % au-dessus du hors ligne en EN et en DE ([ibid.](https://arxiv.org/html/2307.14743)).
- **Statut** : le dépôt se déclare remplacé par SimulStreaming ([README](https://github.com/ufal/whisper_streaming)).

### AlignAtt (SimulStreaming)

- **Principe** : on décode token par token et on s'arrête dès que l'attention encodeur-décodeur du token courant se rapproche de la fin du buffer audio. Le décodage n'a donc pas besoin de deux passes concordantes ([ufal/SimulStreaming](https://github.com/ufal/SimulStreaming)).
- **Gain** : le projet se dit « ~5 fois plus rapide » que whisper_streaming [publié, sans protocole détaillé] ([README](https://github.com/ufal/SimulStreaming)). Le papier explique que LocalAgreement re-traite tout le buffer à chaque chunk, ce qui coûte plus de calcul qu'AlignAtt ([CUNI IWSLT 2025](https://arxiv.org/html/2506.17077v1)).
- **Latences** [publié, matériel non précisé, Whisper large] : en ASR anglais, la latence par mot va de **0,49 s à 2,1 s** selon la configuration (chunks de 0,05 à 2 s) ; en tchèque→anglais, la configuration basse latence est autour de 1,7 s ([ibid.](https://arxiv.org/html/2506.17077v1)).
- **Sur CPU** : le README indique que le code tourne sur CPU mais « trop lentement pour du temps réel », et recommande un GPU d'au moins 10 Go de VRAM ([README](https://github.com/ufal/SimulStreaming)).

### WhisperLive et WhisperLiveKit

- **WhisperLive** (Collabora) : backends faster-whisper, TensorRT et OpenVINO (CPU et iGPU Intel), image Docker CPU, Silero VAD. Le client reçoit des transcriptions *partielles* (`on_partial_transcript`) puis *validées* (`on_committed_transcript`), ce qui correspond exactement au modèle provisoire → définitif ([collabora/WhisperLive](https://github.com/collabora/WhisperLive)). Aucun chiffre de latence n'est publié.
- **WhisperLiveKit** : agrégateur Python qui utilise SimulStreaming/AlignAtt par défaut et LocalAgreement en option. Il couvre Whisper (faster-whisper, MLX), Voxtral, Qwen3-ASR (dont un mode « streaming » qui tourne sur CPU), Canary et SenseVoice, et propose une diarisation Sortformer. Aucun chiffre de latence n'est publié dans le README (benchmarks sur H100 seulement) ([QuentinFuxa/WhisperLiveKit](https://github.com/QuentinFuxa/WhisperLiveKit)).

### Coût du re-décodage

- Whisper encode toujours une fenêtre de 30 s : l'audio est complété par du silence jusqu'à `CHUNK_LENGTH = 30` ([openai/whisper, audio.py](https://github.com/openai/whisper/blob/main/whisper/audio.py)). Avec LocalAgreement, chaque mise à jour relance donc l'encodeur complet sur ces 30 s et le décodeur sur tout le texte du buffer.
- **[estimation]** Avec une mise à jour par seconde, le calcul est de l'ordre de N fois celui d'une transcription hors ligne, N étant le nombre de mises à jour par fenêtre de 30 s : un facteur 10 à 30. C'est cohérent avec l'avertissement de whisper_streaming sur l'API OpenAI, où « chaque fragment audio est traité plusieurs fois » ([README](https://github.com/ufal/whisper_streaming)). Sur un Ryzen 5 7640U sans GPU, seuls les petits Whisper (tiny, base, voire small en int8) ont une chance de tenir un rythme d'une mise à jour par seconde. La latence finale resterait autour de 2 à 4 s, et la qualité serait celle d'un petit modèle.
- Le papier Moonshine v2 donne un repère du coût de Whisper hors ligne : sur un Apple M3, la latence de réponse après la fin de parole est de **289 ms (tiny), 553 ms (base), 1 940 ms (small), 11 286 ms (large-v3)** avec faster-whisper [publié, M3] ([Moonshine v2](https://arxiv.org/html/2602.12241)).

## 4. Moteurs nativement streaming

| Moteur | Langues | Latence algorithmique | Qualité publiée | Exécution locale |
|---|---|---|---|---|
| **Nemotron 3.5 ASR Streaming 0.6B** (NVIDIA, FastConformer cache-aware + RNN-T, sortie 06/2026) | 40 locales, dont **EN, DE et FR en tier « transcription-ready »** | 80, 160, 320, 560 ms ou 1,12 s, choisi à l'inférence | WER FLEURS moyen : 10,4 % (80 ms) → 8,8 % (1,12 s). À 320 ms : EN 8,27, DE 8,83, FR 9,79 [publié] | Licence OpenMDW-1.1 ([carte HF](https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b)) ; pris en charge par **sherpa-onnx** ([PR #3671](https://github.com/k2-fsa/sherpa-onnx/pull/3671), fusionnée le 12/06/2026) et **parakeet-rs** ([docs.rs](https://docs.rs/crate/parakeet-rs/latest)) |
| **Nemotron Speech Streaming EN 0.6B** | EN | 80 ms → 1,12 s | WER Open ASR : 8,43 % (80 ms) → 6,93 % (1,12 s) [publié] | [carte HF](https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b) ; sherpa-onnx, parakeet-rs |
| **Voxtral Mini 4B Realtime 2602** (Mistral) | 13 langues, dont EN, DE, FR | 80 ms → 2,4 s, 480 ms recommandé | WER FLEURS moyen : 12,6 % (160 ms), **8,72 % (480 ms)**, 6,73 % (2,4 s) [publié] | Apache 2.0, au moins 16 Go de GPU en BF16 ; vLLM, Transformers, ports communautaires C et Rust « non testés » ([carte HF](https://huggingface.co/mistralai/Voxtral-Mini-4B-Realtime-2602)) |
| **Kyutai STT 1B** (delayed streams) | **EN + FR** seulement | 0,5 s | — | PyTorch, serveur **Rust** (orienté CUDA), MLX ; poids CC-BY 4.0 ([kyutai-labs/delayed-streams-modeling](https://github.com/kyutai-labs/delayed-streams-modeling)) |
| **Moonshine v2** (Tiny 34M, Small 123M, Medium 245M) | **EN seulement** | Anticipation bornée à 80 ms ; états définitifs après environ 320 ms | WER Open ASR : 12,0 / 7,8 / 6,65 % [publié] | Latence de réponse sur M3 : 50 / 148 / 258 ms [publié] ; C++ / ONNX Runtime, exemple Rust dans sherpa-onnx ([Moonshine v2](https://arxiv.org/html/2602.12241)) |
| **Zipformer streaming** (k2/icefall) | Un modèle par langue : EN et FR existent, **pas de DE** ; bilingue zh-en | Faible, avec des chunks d'environ 320 ms selon le modèle | Pas de RTF publié sur la page | sherpa-onnx, toutes liaisons ([liste des modèles](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html)) |

Points saillants :

- **Le cache-aware élimine le coût du re-décodage** : chaque trame est traitée une seule fois, de façon « strictly non-overlapping », grâce aux caches d'attention et de convolution ([carte Nemotron EN](https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b)). Le coût en calcul est donc proportionnel à la durée de l'audio, comme en hors ligne.
- **Le texte provisoire existe aussi en natif** : Moonshine v2 décode avec des états d'encodeur provisoires sur l'audio le plus récent, puis les remplace par des états définitifs, ce qui améliore le texte affiché ([Moonshine v2](https://arxiv.org/html/2602.12241)).
- **CPU** : la seule indication chiffrée pour Nemotron 3.5 est un **RTF d'environ 0,12 en int8 sur Mac M-series**, relevé par l'auteur de la PR sherpa-onnx en validant son travail. Ce n'est pas un benchmark ([PR #3671](https://github.com/k2-fsa/sherpa-onnx/pull/3671)). **[estimation]** Les 6 cœurs Zen 4 du 7640U devraient tourner dans le même ordre de grandeur, peut-être 1,5 à 3 fois plus lentement, ce qui laisse une marge confortable sous le temps réel avec des chunks de 320 à 560 ms. **Voxtral 4B** n'a pas de chemin CPU documenté : la carte vise un GPU de 16 Go, donc il est peu réaliste sur la machine de référence.

## 5. Détection de la Langue parlée par segment (EN/DE/FR qui alternent)

- **Whisper / faster-whisper** : par défaut, la langue est détectée **sur les 30 premières secondes puis figée**. L'option `multilingual=True` relance la détection « sur chaque segment » ([faster-whisper, transcribe.py](https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/transcribe.py)). WhisperLiveKit note que Whisper en mode auto « penche vers l'anglais » ([README](https://github.com/QuentinFuxa/WhisperLiveKit)).
- **Effet des politiques de streaming** : avec LocalAgreement ou AlignAtt, un buffer court contient peu d'audio. La détection est donc tentée sur quelques secondes au plus, et deux re-décodages successifs peuvent produire deux langues différentes. Le préfixe commun disparaît alors et la latence monte. **[estimation, non mesurée dans les sources]** Le papier whisper_streaming fixe la langue pour chaque évaluation et ne mesure pas la détection automatique ([Macháček et al. 2023](https://arxiv.org/html/2307.14743)).
- **Segmentation par VAD** : c'est le découpage le plus naturel pour avoir **une Langue parlée par énoncé**. On détecte la langue sur l'énoncé complet, ou sur ses premières secondes, puis on la fixe pour le décodage. Une langue qui change au milieu d'un énoncé n'est pas couverte.
- **Nemotron 3.5** : en mode `target_lang=auto`, le modèle ajoute une **étiquette de langue à la fin de chaque énoncé**, après la ponctuation finale. L'auto-détection coûte environ 0,4 à 0,8 point de WER par rapport à une langue imposée (11,14 contre 10,38 % à 80 ms ; 9,21 contre 8,84 % à 1,12 s) [publié]. La carte ne dit rien du changement de langue *à l'intérieur* d'un énoncé ([carte HF](https://huggingface.co/nvidia/nemotron-3.5-asr-streaming-0.6b)). Dans sherpa-onnx, la langue est une option par flux (`"language"`, valeur `auto` possible) et les étiquettes sont retirées du texte ([PR #3671](https://github.com/k2-fsa/sherpa-onnx/pull/3671)). **Conséquence pour le Rendu** : on ne connaît la langue d'un énoncé qu'à sa fin, alors que le texte provisoire s'affiche avant.
- **Autres Moteurs** : Voxtral fait une détection par chunk que WhisperLiveKit juge plus fiable et moins biaisée vers l'anglais. `qwen3-streaming` *exige* une langue explicite, car la détection automatique peut changer de langue en plein flux. Canary s'appuie sur un modèle de LID séparé, AmberNet ([WhisperLiveKit](https://github.com/QuentinFuxa/WhisperLiveKit)). sherpa-onnx propose aussi une identification de langue autonome, ce qui permet un LID par énoncé découplé du Moteur ([Rust API sherpa-onnx](https://k2-fsa.github.io/sherpa/onnx/rust-api/index.html)).

## 6. Implémentations disponibles

| Brique | Python | Rust |
|---|---|---|
| Silero VAD | `silero-vad` (PyTorch / ONNX) | exemples communautaires dans le dépôt ; VAD Silero dans le crate `sherpa-onnx` |
| WebRTC VAD | `py-webrtcvad` | crates de bindings (non vérifiés ici) |
| LocalAgreement / AlignAtt | whisper_streaming, SimulStreaming (torch), WhisperLiveKit, WhisperLive | aucune implémentation de référence trouvée |
| Whisper | faster-whisper, openai-whisper | whisper.cpp (C/C++, bindings Rust communautaires) |
| Transducteurs streaming (Zipformer, Nemotron EN et 3.5) | `sherpa-onnx` | crate **`sherpa-onnx`** (exemples `streaming_zipformer_microphone`, `silero_vad_remove_silence`, `moonshine_v2`…) ([doc Rust](https://k2-fsa.github.io/sherpa/onnx/rust-api/index.html)) ; crate **`parakeet-rs`** (Nemotron EN et 3.5 streaming, EOU, Sortformer, ONNX Runtime CPU par défaut) ([docs.rs](https://docs.rs/crate/parakeet-rs/latest)) |
| VAD + hors ligne en « streaming simulé » | sherpa-onnx | binaire `sherpa-onnx-vad-microphone-simulated-streaming-asr` (Silero + Parakeet TDT, par exemple) ([doc NeMo transducer](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/nemo-transducer-models.html)) |
| Kyutai STT | PyTorch | serveur Rust officiel (orienté GPU) |

sherpa-onnx est en version v2.0.1 (26/10/2026) au moment de la rédaction ([releases](https://github.com/k2-fsa/sherpa-onnx/releases)).

## Ce que ça implique pour la décision

**Synthèse**

1. Avec Whisper, l'objectif « provisoire < 1 s » n'est **pas atteint en pratique**. LocalAgreement donne 3 à 5 s sur GPU A40 ; AlignAtt descend à environ 0,5–2 s par mot mais exige un GPU, et ses auteurs le jugent trop lent sur CPU. Le re-décodage de la fenêtre de 30 s multiplie le calcul, ce qui est rédhibitoire sur un 7640U sans GPU pour un Whisper de bonne qualité.
2. Les **transducteurs cache-aware** répondent directement au besoin : latence algorithmique de 80 à 560 ms, aucun re-calcul, et une famille qui couvre **EN, DE et FR dans un seul modèle** (Nemotron 3.5 Streaming 0.6B, avec étiquette de langue par énoncé). Ce modèle est utilisable dès maintenant en **Python et en Rust** via sherpa-onnx, et en Rust via parakeet-rs. Le seul chiffre CPU est un RTF d'environ 0,12 en int8 sur Mac, non benchmarké.
3. Le schéma **« provisoire puis corrigé »** se construit en deux étages. Un Moteur streaming affiche le provisoire. À la fin de chaque énoncé détecté par VAD (Silero, < 1 ms par chunk sur CPU), on peut soit valider tel quel, soit relancer une seconde passe hors ligne plus précise sur l'énoncé, avec la Langue parlée détectée pour cet énoncé.
4. La Langue parlée se détecte au mieux **par énoncé**, ce qui colle à la segmentation VAD. Les changements de langue *à l'intérieur* d'un énoncé ne sont garantis par aucune solution étudiée.

**Recommandation courte** : prototyper le Moteur live avec **Nemotron 3.5 ASR Streaming 0.6B en int8 via sherpa-onnx**, avec Silero VAD pour les fins d'énoncé, des chunks de 320 ou 560 ms et `language=auto`. Mesurer en premier le RTF et la latence réelle sur le Ryzen 5 7640U, pour remplacer les estimations ci-dessus. Garder une seconde passe hors ligne par énoncé comme option de correction, et écarter les politiques Whisper LocalAgreement/AlignAtt comme chemin principal sur CPU. En repli, si la machine ne tient pas, la piste est Zipformer streaming (EN et FR seulement) ou des chunks plus longs (1,12 s).
