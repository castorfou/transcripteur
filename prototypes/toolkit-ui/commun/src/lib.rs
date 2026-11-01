//! PROTOTYPE jetable : flux simulé et mesures partagés par les trois essais.
//!
//! Le flux tourne dans un thread de fond et réveille l'UI, comme le fera le Moteur :
//! la latence mesurée va du `tick()` du thread jusqu'à la frame qui affiche le texte.

use std::collections::HashSet;
use std::fs;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const PHRASES: &[&str] = &[
    "Thank you all for being here at this conference in Berlin today",
    "Accelerated computing is reshaping how every data centre is designed and operated",
    "Wir freuen uns sehr, heute über die Größe unserer Modelle zu sprechen",
    "Die Übertragung läuft über mehrere Knoten und bleibt dabei äußerst zuverlässig",
    "Nous allons maintenant détailler les résultats obtenus sur la dernière génération",
    "Ça reste très encourageant, même si les données restent à vérifier à froid",
    "The next slide shows the latency distribution measured across all our workloads",
    "Künstliche Intelligenz verändert gerade die Art, wie Fabriken geplant werden",
    "Cette évolution concerne aussi bien la recherche que l'industrie européenne",
    "So let's take a closer look at what happens when the model runs on the edge",
];

const LANGUES: &[&str] =
    &["EN", "EN", "DE", "DE", "FR", "FR", "EN", "DE", "FR", "EN"];

/// Période de décodage du Moteur retenu (Nemotron 320 ms).
pub const PERIODE_TICK: Duration = Duration::from_millis(320);
/// Une Session d'1 h à un énoncé toutes les 2,4 s.
pub const ENONCES_1H: usize = 1500;

#[derive(Clone, Debug)]
pub struct Mot {
    pub texte: String,
    pub modifie: bool,
}

#[derive(Clone, Debug)]
pub struct Enonce {
    pub debut_s: u32,
    pub langue: &'static str,
    pub mots: Vec<Mot>,
}

impl Enonce {
    fn depuis_phrase(index: usize) -> Self {
        let phrase = PHRASES[index % PHRASES.len()];
        let mots = phrase
            .split(' ')
            .enumerate()
            .map(|(i, mot)| Mot { texte: mot.to_owned(), modifie: (i + index) % 5 == 0 })
            .collect();
        let langue = LANGUES[index % LANGUES.len()];
        Self { debut_s: (index as f32 * 2.4) as u32, langue, mots }
    }

    pub fn horodatage(&self) -> String {
        let s = self.debut_s;
        format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    }

    pub fn texte(&self) -> String {
        self.mots.iter().map(|m| m.texte.as_str()).collect::<Vec<_>>().join(" ")
    }
}

pub struct Flux {
    pub courante: &'static str,
    pub historique: Vec<Enonce>,
    pub dernier: Enonce,
    pub provisoire: Vec<String>,
    /// Incrémenté à chaque tick : l'UI s'en sert pour ne relire que ce qui change.
    pub version: u64,
    index: usize,
    courant: Enonce,
}

impl Flux {
    fn new() -> Self {
        let historique: Vec<_> = (0..ENONCES_1H).map(Enonce::depuis_phrase).collect();
        let dernier = historique[ENONCES_1H - 1].clone();
        Self {
            courante: dernier.langue,
            historique,
            dernier,
            provisoire: Vec::new(),
            version: 0,
            index: ENONCES_1H,
            courant: Enonce::depuis_phrase(ENONCES_1H),
        }
    }

    fn courante_enonce(&self) -> &'static str {
        if self.provisoire.is_empty() { self.dernier.langue } else { self.courant.langue }
    }

    fn avancer(&mut self) {
        self.version += 1;
        self.courante = self.courante_enonce();
        if self.provisoire.len() < self.courant.mots.len() {
            let mot = &self.courant.mots[self.provisoire.len()];
            let texte = if mot.modifie { mot.texte.to_lowercase() } else { mot.texte.clone() };
            self.provisoire.push(texte);
            return;
        }
        self.index += 1;
        let suivant = Enonce::depuis_phrase(self.index);
        let fini = std::mem::replace(&mut self.courant, suivant);
        self.dernier = fini.clone();
        self.historique.push(fini);
        self.provisoire.clear();
    }
}

/// Démarre le flux simulé ; `reveil` est appelé depuis le thread de fond après chaque tick.
pub fn demarrer_flux(reveil: impl Fn() + Send + 'static) -> Arc<Mutex<Flux>> {
    let flux = Arc::new(Mutex::new(Flux::new()));
    let copie = flux.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(PERIODE_TICK);
            copie.lock().unwrap().avancer();
            sonde().lock().unwrap().tick();
            reveil();
        }
    });
    flux
}

#[derive(Default)]
struct Phase {
    latences_tick_ms: Vec<f32>,
    intervalles_frame_ms: Vec<f32>,
}

pub struct Sonde {
    nom: String,
    debut: Instant,
    tick_en_attente: Option<Instant>,
    derniere_frame: Option<Instant>,
    repos: Phase,
    defilement: Phase,
    cpu_debut: (f64, Instant),
    cpu_pourcent: f32,
    cpu_phase: (f64, Instant),
    cpu_repos: Option<f32>,
    dernier_releve: Instant,
    ligne: String,
    auto: bool,
    notes: Vec<String>,
}

const REPOS_S: f32 = 15.0;
const DEFILEMENT_S: f32 = 30.0;
const ALLER_RETOUR_S: f32 = 20.0;

fn sonde_globale() -> &'static Mutex<Sonde> {
    static SONDE: OnceLock<Mutex<Sonde>> = OnceLock::new();
    SONDE.get_or_init(|| {
        let auto = std::env::args().any(|a| a == "--auto");
        let maintenant = Instant::now();
        Mutex::new(Sonde {
            nom: String::new(),
            debut: maintenant,
            tick_en_attente: None,
            derniere_frame: None,
            repos: Phase::default(),
            defilement: Phase::default(),
            cpu_debut: (temps_cpu_s(), maintenant),
            cpu_pourcent: 0.0,
            cpu_phase: (temps_cpu_s(), maintenant),
            cpu_repos: None,
            dernier_releve: maintenant,
            ligne: String::new(),
            auto,
            notes: Vec::new(),
        })
    })
}

pub fn sonde() -> &'static Mutex<Sonde> {
    sonde_globale()
}

pub fn initialiser(nom: &str) {
    sonde().lock().unwrap().nom = nom.to_owned();
    if mode_auto() {
        // Filet : un rendu qui ne remonte pas ses frames ne doit pas bloquer le scénario.
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs_f32(REPOS_S + DEFILEMENT_S + 3.0));
            quitter();
        });
    }
}

/// En `--auto`, position de défilement voulue dans l'historique (0 = haut, 1 = bas),
/// `None` hors de la phase de défilement.
pub fn defilement_auto() -> Option<f32> {
    let s = sonde().lock().unwrap();
    let t = s.debut.elapsed().as_secs_f32() - REPOS_S;
    if !s.auto || t < 0.0 {
        return None;
    }
    let cycle = (t / ALLER_RETOUR_S).fract() * 2.0;
    Some(if cycle < 1.0 { 1.0 - cycle } else { cycle - 1.0 })
}

pub fn mode_auto() -> bool {
    sonde().lock().unwrap().auto
}

pub fn ligne_mesures() -> String {
    sonde().lock().unwrap().ligne.clone()
}

/// Résultat d'un essai à consigner dans le rapport : `note("deplier", "...")`.
pub fn note(cle: &str, valeur: &str) {
    let texte = format!("{cle}={valeur}");
    println!("NOTE {texte}");
    sonde().lock().unwrap().notes.push(texte);
}

pub fn quitter() -> ! {
    println!("{}", sonde().lock().unwrap().rapport());
    std::process::exit(0);
}

impl Sonde {
    fn tick(&mut self) {
        self.tick_en_attente = Some(Instant::now());
    }

    /// À appeler une fois par frame réellement dessinée.
    pub fn frame(&mut self) {
        let maintenant = Instant::now();
        let t = self.debut.elapsed().as_secs_f32();
        if self.auto && t >= REPOS_S && self.cpu_repos.is_none() {
            self.cpu_repos = Some(self.cpu_moyen_phase());
            self.cpu_phase = (temps_cpu_s(), maintenant);
        }
        let phase = if self.auto && t >= REPOS_S { &mut self.defilement } else { &mut self.repos };
        if let Some(emis) = self.tick_en_attente.take() {
            phase.latences_tick_ms.push((maintenant - emis).as_secs_f32() * 1000.0);
        }
        if let Some(avant) = self.derniere_frame {
            phase.intervalles_frame_ms.push((maintenant - avant).as_secs_f32() * 1000.0);
        }
        self.derniere_frame = Some(maintenant);

        if self.dernier_releve.elapsed() >= Duration::from_secs(1) {
            self.relever();
            self.dernier_releve = maintenant;
        }
        if self.auto && t >= REPOS_S + DEFILEMENT_S {
            println!("{}", self.rapport());
            std::process::exit(0);
        }
    }

    fn cpu_moyen_phase(&self) -> f32 {
        let (cpu, instant) = self.cpu_phase;
        ((temps_cpu_s() - cpu) / instant.elapsed().as_secs_f64() * 100.0) as f32
    }

    fn relever(&mut self) {
        let (cpu, instant) = self.cpu_debut;
        let cpu_maintenant = temps_cpu_s();
        self.cpu_pourcent =
            ((cpu_maintenant - cpu) / instant.elapsed().as_secs_f64() * 100.0) as f32;
        self.cpu_debut = (cpu_maintenant, Instant::now());
        let lat = centiles(&self.repos.latences_tick_ms);
        self.ligne = format!(
            "CPU {:.0} % · RAM {:.0} Mo · GPU {:.0} Mo · tick p95 {:.0} ms",
            self.cpu_pourcent,
            rss_mo(),
            gpu_mo(),
            lat.1
        );
    }

    fn rapport(&mut self) -> String {
        self.relever();
        let format_phase = |p: &Phase| {
            let l = centiles(&p.latences_tick_ms);
            let f = centiles(&p.intervalles_frame_ms);
            format!(
                "tick_ms(p50={:.1},p95={:.1},max={:.1}) frame_ms(p50={:.1},p95={:.1},max={:.1},n={})",
                l.0,
                l.1,
                l.2,
                f.0,
                f.1,
                f.2,
                p.intervalles_frame_ms.len() + 1
            )
        };
        format!(
            "RESULTAT {} session={} wayland={} repos[{}] defilement[{}] cpu_repos={:.1}% cpu_defilement={:.1}% rss_mo={:.0} gpu_mo={:.0} notes[{}]",
            self.nom,
            std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
            std::env::var_os("WAYLAND_DISPLAY").is_some(),
            format_phase(&self.repos),
            format_phase(&self.defilement),
            self.cpu_repos.unwrap_or(0.0),
            self.cpu_moyen_phase(),
            rss_mo(),
            gpu_mo(),
            self.notes.join("; ")
        )
    }
}

fn centiles(valeurs: &[f32]) -> (f32, f32, f32) {
    if valeurs.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut v = valeurs.to_vec();
    v.sort_by(f32::total_cmp);
    (v[v.len() / 2], v[v.len() * 95 / 100], v[v.len() - 1])
}

fn temps_cpu_s() -> f64 {
    let stat = fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let champs: Vec<&str> = stat.rsplit(')').next().unwrap_or("").split_whitespace().collect();
    let tics = |i: usize| champs.get(i).and_then(|c| c.parse::<f64>().ok()).unwrap_or(0.0);
    (tics(11) + tics(12)) / 100.0
}

fn rss_mo() -> f32 {
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|l| l.split_whitespace().next()?.parse::<f32>().ok())
        .unwrap_or(0.0)
        / 1024.0
}

/// Mémoire GPU du processus (VRAM + GTT) d'après `/proc/self/fdinfo`, clients DRM dédoublonnés.
fn gpu_mo() -> f32 {
    let mut clients = HashSet::new();
    let mut total_kio = 0.0;
    let Ok(fds) = fs::read_dir("/proc/self/fdinfo") else { return 0.0 };
    for fd in fds.flatten() {
        let Ok(info) = fs::read_to_string(fd.path()) else { continue };
        let client = info.lines().find_map(|l| l.strip_prefix("drm-client-id:"));
        let Some(client) = client else { continue };
        if !clients.insert(client.trim().to_owned()) {
            continue;
        }
        for ligne in info.lines() {
            if ligne.starts_with("drm-memory-vram:") || ligne.starts_with("drm-memory-gtt:") {
                let valeur = ligne.split_whitespace().nth(1);
                total_kio += valeur.and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
            }
        }
    }
    total_kio / 1024.0
}
