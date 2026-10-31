//! PROTOTYPE jetable : Spike de bout en bout sur le Ryzen.
//!
//! Une seule chaîne de bout en bout, nourrie par un WAV (Banc d'essai) ou par une
//! capture (Source) :
//!
//!   audio -> [thread live : Moteur streaming + VAD] -> Texte provisoire
//!                  |
//!                  +-- énoncé fini --> [thread seconde passe : langue + Moteur]
//!                                                -> Texte définitif
//!
//! Sortie : un JSONL d'événements horodatés, scoré par `scripts/spike_score.py`.
//! Pas de gestion d'erreur soignée : `expect` suffit, on veut apprendre vite.

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rubato::{FftFixedIn, Resampler};
use serde_json::{Value, json};
use sherpa_onnx::{
    OfflineCanaryModelConfig, OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig,
    OnlineModelConfig, OnlineRecognizer, OnlineRecognizerConfig, OnlineTransducerModelConfig,
    SileroVadModelConfig, SpokenLanguageIdentification, SpokenLanguageIdentificationConfig,
    SpokenLanguageIdentificationWhisperConfig, VadModelConfig, VoiceActivityDetector, Wave,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const SR: usize = 16_000;
/// Silero attend des fenêtres de 512 échantillons (32 ms) : c'est aussi notre taille
/// de bloc partout, ce qui évite de redécouper l'audio.
const BLOC: usize = 512;
const MODELES: &str = "data/raw/models";

/// Options de la ligne de commande : `--cle valeur`, sans dépendance à clap.
/// `HashMap<String, String>` plutôt qu'une struct : c'est un prototype, et on
/// préfère ajouter une option en une ligne.
struct Opts(HashMap<String, String>);

impl Opts {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        // `chunks(2)` donne des tranches `&[String]` de deux éléments : `--cle`, `valeur`.
        let paires = args
            .chunks(2)
            .map(|c| (c[0].trim_start_matches("--").to_string(), c[1].clone()));
        Opts(paires.collect())
    }

    /// Valeur de l'option, ou `defaut`. Prendre `&str` en entrée accepte un littéral
    /// comme une `String` empruntée, sans copie à l'appel.
    fn get(&self, cle: &str, defaut: &str) -> String {
        self.0.get(cle).cloned().unwrap_or(defaut.to_string())
    }

    fn num<T: std::str::FromStr>(&self, cle: &str, defaut: &str) -> T {
        self.get(cle, defaut).parse().ok().expect(cle)
    }
}

/// Journal partagé entre threads. `Arc` donne plusieurs propriétaires, `Mutex`
/// l'accès exclusif : à eux deux, ils remplacent le ramasse-miettes et le verrou
/// qu'on aurait en Python, et le compilateur refuse un accès sans verrou.
#[derive(Clone)]
struct Journal {
    t0: Arc<Mutex<Option<Instant>>>,
    sortie: Arc<Mutex<BufWriter<File>>>,
    afficher: bool,
}

impl Journal {
    fn demarrer(&self) {
        *self.t0.lock().unwrap() = Some(Instant::now());
        // Heure absolue : recale une capture sur l'instant où la vidéo a été lancée.
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        self.ecrire(json!({"ev": "depart", "unix": unix}));
    }

    fn t(&self) -> f64 {
        self.t0.lock().unwrap().map_or(0.0, |d| d.elapsed().as_secs_f64())
    }

    fn ecrire(&self, mut ev: Value) {
        ev["t"] = json!(self.t());
        writeln!(self.sortie.lock().unwrap(), "{ev}").unwrap();
    }
}

/// Mémoire résidente courante et pic, en Mo, lus dans /proc.
fn memoire_mo() -> (u64, u64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let champ = |nom: &str| -> u64 {
        status
            .lines()
            .find(|l| l.starts_with(nom))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            / 1024
    };
    (champ("VmRSS:"), champ("VmHWM:"))
}

fn dossier(motif: &str) -> String {
    // On cherche le dossier du modèle plutôt que de recopier la date de son nom.
    std::fs::read_dir(MODELES)
        .unwrap()
        .map(|e| e.unwrap().path().display().to_string())
        .find(|p| p.contains(motif))
        .unwrap_or_else(|| panic!("modèle absent : {motif}"))
}

fn moteur_live(motif: &str, threads: i32) -> OnlineRecognizer {
    let d = dossier(motif);
    let cfg = OnlineRecognizerConfig {
        model_config: OnlineModelConfig {
            transducer: OnlineTransducerModelConfig {
                encoder: Some(format!("{d}/encoder.int8.onnx")),
                decoder: Some(format!("{d}/decoder.int8.onnx")),
                joiner: Some(format!("{d}/joiner.int8.onnx")),
            },
            tokens: Some(format!("{d}/tokens.txt")),
            num_threads: threads,
            ..Default::default()
        },
        decoding_method: Some("greedy_search".into()),
        enable_endpoint: true,
        rule1_min_trailing_silence: 2.0,
        rule2_min_trailing_silence: 0.5,
        rule3_min_utterance_length: 20.0,
        ..Default::default()
    };
    OnlineRecognizer::create(&cfg).expect("Moteur live")
}

fn vad(min_silence: f32, max_enonce: f32) -> VoiceActivityDetector {
    let cfg = VadModelConfig {
        silero_vad: SileroVadModelConfig {
            model: Some(format!("{MODELES}/silero_vad.onnx")),
            threshold: 0.5,
            min_silence_duration: min_silence,
            min_speech_duration: 0.25,
            window_size: BLOC as i32,
            max_speech_duration: max_enonce,
        },
        sample_rate: SR as i32,
        num_threads: 1,
        ..Default::default()
    };
    VoiceActivityDetector::create(&cfg, 120.0).expect("VAD")
}

/// Le Moteur de seconde passe. Un `enum` ferme la liste des variantes et oblige
/// chaque `match` à les traiter toutes : ajouter un Moteur signale tous les oublis
/// à la compilation.
enum Seconde {
    Aucune,
    Parakeet(OfflineRecognizer),
    Canary(OfflineRecognizer),
    Whisper(WhisperContext),
}

struct Decode {
    texte: String,
    /// (token, début, durée) en secondes depuis le début de l'énoncé.
    mots: Vec<(String, f32, f32)>,
}

impl Seconde {
    fn charger(nom: &str, threads: i32) -> Self {
        match nom {
            "aucune" => Seconde::Aucune,
            "parakeet" => {
                let d = dossier("parakeet-tdt-0.6b-v3");
                let mut cfg = OfflineRecognizerConfig::default();
                cfg.model_config = OfflineModelConfig {
                    transducer: OfflineTransducerModelConfig {
                        encoder: Some(format!("{d}/encoder.int8.onnx")),
                        decoder: Some(format!("{d}/decoder.int8.onnx")),
                        joiner: Some(format!("{d}/joiner.int8.onnx")),
                    },
                    tokens: Some(format!("{d}/tokens.txt")),
                    num_threads: threads,
                    model_type: Some("nemo_transducer".into()),
                    ..Default::default()
                };
                cfg.decoding_method = Some("greedy_search".into());
                Seconde::Parakeet(OfflineRecognizer::create(&cfg).expect("Parakeet"))
            }
            "canary" => {
                let d = dossier("canary-180m-flash");
                let mut cfg = OfflineRecognizerConfig::default();
                cfg.model_config.canary = OfflineCanaryModelConfig {
                    encoder: Some(format!("{d}/encoder.int8.onnx")),
                    decoder: Some(format!("{d}/decoder.int8.onnx")),
                    src_lang: Some("en".into()),
                    tgt_lang: Some("en".into()),
                    use_pnc: true,
                };
                cfg.model_config.tokens = Some(format!("{d}/tokens.txt"));
                cfg.model_config.num_threads = threads;
                Seconde::Canary(OfflineRecognizer::create(&cfg).expect("Canary"))
            }
            "whisper" => {
                let chemin = format!("{MODELES}/ggml-large-v3-turbo-q5_0.bin");
                let ctx = WhisperContext::new_with_params(
                    &chemin,
                    WhisperContextParameters::default(),
                )
                .expect("Whisper");
                Seconde::Whisper(ctx)
            }
            autre => panic!("seconde passe inconnue : {autre}"),
        }
    }

    fn decoder(&self, audio: &[f32], langue: &str, threads: i32) -> Option<Decode> {
        match self {
            Seconde::Aucune => None,
            Seconde::Parakeet(r) => {
                let s = r.create_stream();
                s.accept_waveform(SR as i32, audio);
                r.decode(&s);
                let res = s.get_result().expect("résultat Parakeet");
                let debuts = res.timestamps.unwrap_or_default();
                let durees = res.durations.unwrap_or_default();
                let mots = res
                    .tokens
                    .into_iter()
                    .zip(debuts)
                    .zip(durees)
                    .map(|((tok, debut), duree)| (tok, debut, duree))
                    .collect();
                Some(Decode { texte: res.text, mots })
            }
            Seconde::Canary(r) => {
                let s = r.create_stream();
                // Langue par flux : transcrire dans la langue parlée, jamais traduire.
                s.set_option("src_lang", langue);
                s.set_option("tgt_lang", langue);
                s.accept_waveform(SR as i32, audio);
                r.decode(&s);
                let res = s.get_result().expect("résultat Canary");
                Some(Decode { texte: res.text, mots: vec![] })
            }
            Seconde::Whisper(ctx) => {
                let mut etat = ctx.create_state().expect("état Whisper");
                let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                p.set_language(Some(langue));
                p.set_n_threads(threads);
                p.set_print_progress(false);
                p.set_print_realtime(false);
                p.set_print_special(false);
                // Chaque énoncé est décodé seul : pas de contexte entre énoncés.
                p.set_no_context(true);
                // L'encodeur coûte selon sa fenêtre, pas selon l'audio : 750 = 15 s, soit
                // la durée maximale d'un énoncé, au lieu des 30 s par défaut.
                p.set_audio_ctx(
                    std::env::var("SPIKE_AUDIO_CTX").map_or(0, |v| v.parse().expect("ctx")),
                );
                etat.full(p, audio).expect("décodage Whisper");
                let mut texte = String::new();
                for i in 0..etat.full_n_segments() {
                    let seg = etat.get_segment(i).expect("segment Whisper");
                    texte.push_str(&seg.to_str_lossy().expect("texte Whisper"));
                }
                Some(Decode { texte: texte.trim().to_string(), mots: vec![] })
            }
        }
    }
}

fn detecteur_langue() -> SpokenLanguageIdentification {
    let d = dossier("whisper-tiny");
    let cfg = SpokenLanguageIdentificationConfig {
        whisper: SpokenLanguageIdentificationWhisperConfig {
            encoder: Some(format!("{d}/tiny-encoder.int8.onnx")),
            decoder: Some(format!("{d}/tiny-decoder.int8.onnx")),
            tail_paddings: 0,
        },
        num_threads: 2,
        ..Default::default()
    };
    SpokenLanguageIdentification::create(&cfg).expect("détecteur de langue")
}

struct Enonce {
    debut: f64,
    audio: Vec<f32>,
}

/// Thread de seconde passe : reçoit les énoncés finis, écrit le Texte définitif.
/// Le canal est borné par rien : s'il grossit, la colonne `attente` le montre.
fn seconde_passe(
    rx: Receiver<Enonce>,
    journal: Journal,
    nom: String,
    threads: i32,
    lid: bool,
    pret: Arc<Barrier>,
) {
    let moteur = Seconde::charger(&nom, threads);
    let lid = lid.then(detecteur_langue);
    pret.wait();
    let mut langue_retenue = "en".to_string();
    // `for ... in rx` s'arrête quand tous les `Sender` sont détruits : fermer le canal
    // suffit à arrêter ce thread, sans drapeau partagé.
    for e in rx {
        let debut = Instant::now();
        let brute = lid.as_ref().map_or("?".to_string(), |l| {
            let s = l.create_stream();
            s.accept_waveform(SR as i32, &e.audio[..e.audio.len().min(10 * SR)]);
            l.compute(&s).map_or("?".to_string(), |r| r.lang)
        });
        // Hors des langues visées, c'est du bruit du détecteur : on garde la précédente.
        if ["en", "de", "fr"].contains(&brute.as_str()) {
            langue_retenue = brute.clone();
        }
        let langue = langue_retenue.clone();
        let t_lid = debut.elapsed().as_secs_f64();
        let res = moteur.decoder(&e.audio, &langue, threads);
        let duree = e.audio.len() as f64 / SR as f64;
        // `Option::map_or` évite un `match` pour le cas « pas de seconde passe ».
        let (texte, mots) = res.map_or((String::new(), vec![]), |d| (d.texte, d.mots));
        journal.ecrire(json!({
            "ev": "definitif", "debut": e.debut, "fin": e.debut + duree, "langue": langue,
            "brute": brute, "t_lid": t_lid, "t_calcul": debut.elapsed().as_secs_f64(), "texte": texte,
            "mots": mots,
        }));
        if journal.afficher {
            println!("\n[{langue}] {texte}");
        }
    }
}

struct Reglages {
    moteur: String,
    langue: String,
    threads: i32,
    min_silence: f32,
    max_enonce: f32,
}

/// Thread live : Moteur streaming et VAD. Retourne le temps de calcul cumulé.
fn live(
    rx: Receiver<Vec<f32>>,
    vers_seconde: Sender<Enonce>,
    journal: Journal,
    r: Reglages,
    pret: Arc<Barrier>,
) -> (f64, f64) {
    let moteur = moteur_live(&r.moteur, r.threads);
    let detecteur = vad(r.min_silence, r.max_enonce);
    let flux = moteur.create_stream();
    flux.set_option("language", &r.langue);
    pret.wait();
    journal.demarrer();

    let (mut lu, mut debut_enonce, mut deja) = (0usize, 0.0f64, 0usize);
    let (mut calcul_moteur, mut calcul_vad) = (0.0f64, 0.0f64);
    let mut prochain_sys = 0.0;
    let mut texte_courant = String::new();

    // Ferme l'énoncé du Moteur live : c'est le Texte définitif sans seconde passe.
    let clore = |texte: &str, debut: f64, fin: f64| {
        if !texte.is_empty() {
            journal.ecrire(json!({"ev": "fin_live", "debut": debut, "fin": fin, "texte": texte}));
        }
    };

    while let Ok(bloc) = rx.recv() {
        lu += bloc.len();
        let pos = lu as f64 / SR as f64;

        let c = Instant::now();
        detecteur.accept_waveform(&bloc);
        calcul_vad += c.elapsed().as_secs_f64();
        while let Some(seg) = detecteur.front() {
            let e = Enonce {
                debut: seg.start() as f64 / SR as f64,
                audio: seg.samples().to_vec(),
            };
            detecteur.pop();
            vers_seconde.send(e).unwrap();
        }

        let c = Instant::now();
        flux.accept_waveform(SR as i32, &bloc);
        while moteur.is_ready(&flux) {
            moteur.decode(&flux);
        }
        calcul_moteur += c.elapsed().as_secs_f64();

        if let Some(res) = moteur.get_result(&flux) {
            if res.tokens.len() > deja {
                let stamps = res.timestamps.unwrap_or_default();
                // Le transducteur n'efface jamais : seuls les nouveaux tokens comptent.
                let nouveaux: Vec<_> = res.tokens[deja..]
                    .iter()
                    .zip(stamps.iter().skip(deja))
                    .map(|(tok, ts)| (tok.clone(), debut_enonce + *ts as f64))
                    .collect();
                journal.ecrire(json!({"ev": "provisoire", "a": pos, "nouveaux": nouveaux}));
                if journal.afficher {
                    print!("\r\x1b[2K{}", dernier_caracteres(&res.text, 100));
                    std::io::stdout().flush().unwrap();
                }
                deja = res.tokens.len();
                texte_courant = res.text;
            }
        }

        if moteur.is_endpoint(&flux) {
            clore(&texte_courant, debut_enonce, pos);
            moteur.reset(&flux);
            flux.set_option("language", &r.langue);
            texte_courant.clear();
            debut_enonce = pos;
            deja = 0;
        }

        if journal.t() >= prochain_sys {
            let (rss, pic) = memoire_mo();
            journal.ecrire(json!({"ev": "sys", "a": pos, "rss": rss, "pic": pic,
                "moteur": calcul_moteur, "vad": calcul_vad}));
            prochain_sys += 10.0;
        }
    }

    // Fin de l'audio : on vide le Moteur et le VAD, sinon le dernier énoncé se perd.
    flux.accept_waveform(SR as i32, &vec![0.0; SR]);
    flux.input_finished();
    while moteur.is_ready(&flux) {
        moteur.decode(&flux);
    }
    if let Some(res) = moteur.get_result(&flux) {
        clore(&res.text, debut_enonce, lu as f64 / SR as f64);
    }
    detecteur.flush();
    while let Some(seg) = detecteur.front() {
        let e = Enonce { debut: seg.start() as f64 / SR as f64, audio: seg.samples().to_vec() };
        detecteur.pop();
        vers_seconde.send(e).unwrap();
    }
    (calcul_moteur, calcul_vad)
}

/// Les `n` derniers caractères : une tranche `&s[..]` se coupe en octets et paniquerait
/// au milieu d'un « é », d'où le passage par `char_indices`.
fn dernier_caracteres(s: &str, n: usize) -> &str {
    let total = s.chars().count();
    let debut = s.char_indices().nth(total.saturating_sub(n)).map_or(0, |(i, _)| i);
    &s[debut..]
}

/// Banc d'essai : envoie un WAV au rythme demandé. `vitesse` = 0 : sans attendre.
fn banc(chemin: &str, vitesse: f64, tx: Sender<Vec<f32>>) -> f64 {
    let wav = Wave::read(chemin).expect("lecture du WAV");
    assert_eq!(wav.sample_rate() as usize, SR);
    let depart = Instant::now();
    for (i, bloc) in wav.samples().chunks(BLOC).enumerate() {
        if vitesse > 0.0 {
            // On vise une heure absolue plutôt que `sleep(32 ms)` à chaque tour :
            // les petites erreurs de sommeil ne s'additionnent pas.
            let cible = ((i + 1) * BLOC) as f64 / SR as f64 / vitesse;
            let ecoule = depart.elapsed().as_secs_f64();
            if cible > ecoule {
                std::thread::sleep(Duration::from_secs_f64(cible - ecoule));
            }
        }
        tx.send(bloc.to_vec()).unwrap();
    }
    wav.num_samples() as f64 / SR as f64
}

/// Source réelle : capte l'entrée par défaut pendant `secondes`. La Source visée se
/// choisit avec la variable `PULSE_SOURCE` (micro ou `.monitor` pour le Son système) :
/// dans le conteneur, ALSA est redirigé vers le PipeWire de l'hôte.
fn capture(secondes: f64, tx: Sender<Vec<f32>>) -> f64 {
    let hote = cpal::default_host();
    let entree = hote.default_input_device().expect("pas d'entrée audio");
    let cfg = entree.default_input_config().expect("config d'entrée");
    let (voies, taux) = (cfg.channels() as usize, cfg.sample_rate() as usize);
    eprintln!("capture : {taux} Hz, {voies} voies, {:?}", cfg.sample_format());
    assert_eq!(cfg.sample_format(), SampleFormat::F32, "format non géré");

    let mut reech = FftFixedIn::<f32>::new(taux, SR, 1024, 2, 1).expect("rééchantillonneur");
    let (mut entrant, mut sortant) = (Vec::<f32>::new(), Vec::<f32>::new());
    // La closure est appelée par le thread audio : `move` lui donne la propriété de
    // tout ce qu'elle utilise, car elle vit plus longtemps que cette fonction.
    let flux = entree
        .build_input_stream::<f32, _, _>(
            cfg.into(),
            move |data: &[f32], _: &_| {
                // Mono par moyenne des voies.
                entrant.extend(data.chunks(voies).map(|c| c.iter().sum::<f32>() / voies as f32));
                while entrant.len() >= reech.input_frames_next() {
                    let n = reech.input_frames_next();
                    let sortie = reech.process(&[&entrant[..n]], None).expect("rééchantillonnage");
                    entrant.drain(..n);
                    sortant.extend_from_slice(&sortie[0]);
                }
                while sortant.len() >= BLOC {
                    tx.send(sortant.drain(..BLOC).collect()).ok();
                }
            },
            |e| eprintln!("erreur de capture : {e}"),
            None,
        )
        .expect("flux d'entrée");
    flux.play().expect("démarrage de la capture");
    std::thread::sleep(Duration::from_secs_f64(secondes));
    // `flux` est détruit en sortant : la closure, donc `tx`, l'est aussi, et le canal
    // se ferme comme à la fin d'un WAV.
    secondes
}

fn main() {
    let o = Opts::parse();
    let reglages = Reglages {
        moteur: o.get("moteur", "nemotron-3.5-asr-streaming-0.6b-320ms"),
        langue: o.get("langue", "auto"),
        threads: o.num("threads", "4"),
        min_silence: o.num("min-silence", "0.5"),
        max_enonce: o.num("max-enonce", "15"),
    };
    let journal = Journal {
        t0: Arc::new(Mutex::new(None)),
        sortie: Arc::new(Mutex::new(BufWriter::new(
            File::create(o.get("sortie", "/dev/stdout")).expect("fichier de sortie"),
        ))),
        afficher: o.get("afficher", "non") == "oui",
    };
    journal.ecrire(json!({"ev": "config", "args": o.0}));

    let (tx_audio, rx_audio) = mpsc::channel::<Vec<f32>>();
    let (tx_enonce, rx_enonce) = mpsc::channel::<Enonce>();
    // Trois attendent le chargement des modèles avant de démarrer l'horloge.
    let pret = Arc::new(Barrier::new(3));

    let th_seconde = std::thread::spawn({
        // Un bloc qui clone puis déplace : idiome courant pour donner à un thread sa
        // propre poignée sans perdre celle du thread principal.
        let (journal, pret) = (journal.clone(), pret.clone());
        let (nom, threads, lid) = (
            o.get("seconde", "aucune"),
            o.num("threads-seconde", "4"),
            o.get("lid", "oui") == "oui",
        );
        move || seconde_passe(rx_enonce, journal, nom, threads, lid, pret)
    });
    let th_live = std::thread::spawn({
        let (journal, pret) = (journal.clone(), pret.clone());
        move || live(rx_audio, tx_enonce, journal, reglages, pret)
    });

    pret.wait();
    let duree = match o.0.get("capture") {
        Some(secondes) => capture(secondes.parse().expect("capture"), tx_audio),
        None => banc(&o.get("wav", ""), o.num("vitesse", "1"), tx_audio),
    };
    // `tx_audio` a été déplacé dans la Source : à son retour le canal se ferme,
    // ce qui arrête le thread live, puis en cascade la seconde passe.
    let (calcul_moteur, calcul_vad) = th_live.join().unwrap();
    th_seconde.join().unwrap();

    let (_, pic) = memoire_mo();
    journal.ecrire(json!({"ev": "bilan", "duree": duree, "moteur": calcul_moteur,
        "vad": calcul_vad, "pic": pic}));
    journal.sortie.lock().unwrap().flush().unwrap();
}
