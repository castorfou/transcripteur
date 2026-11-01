//! PROTOTYPE jetable : essai de Slint pour la forme de l'affichage live.

use commun::Flux;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, Model, ModelRc, Timer, TimerMode, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const HAUTEUR_COMPACTE: f32 = 232.0;
const HAUTEUR_HISTORIQUE: f32 = 560.0;

static FLUX: OnceLock<Arc<Mutex<Flux>>> = OnceLock::new();

slint::slint! {
    import { ListView } from "std-widgets.slint";
    import "polices/Atkinson-Regular.ttf";
    import "polices/Atkinson-Italic.ttf";
    import "polices/Atkinson-Bold.ttf";
    import "polices/Atkinson-BoldItalic.ttf";
    import "polices/JetBrainsMono.ttf";

    export struct Ligne { h: string, t: string, l: string }

    // Valeurs reprises de la variante D de la maquette validée.
    global Theme {
        out property <color> texte: #f5f7f8;
        out property <color> attenue: #9fb0bc;
        out property <color> ligne: #33414c;
        out property <color> accent: #6fd0ee;
        out property <color> sur-accent: #06222c;
        out property <color> alerte: #ffc56b;
        out property <string> mono: "JetBrains Mono";
    }

    component Bouton {
        in property <string> texte;
        in property <bool> actif: false;
        in property <length> taille: 12px;
        callback clic();
        min-width: libelle.preferred-width + 14px;
        horizontal-stretch: 0;
        Rectangle {
            border-radius: 3px;
            border-width: 1px;
            border-color: root.actif ? Theme.accent : Theme.ligne;
            background: root.actif ? Theme.accent
                : zone.has-hover ? #ffffff1a : #00000040;
            libelle := Text {
                text: root.texte;
                font-size: root.taille;
                color: root.actif ? Theme.sur-accent : Theme.texte;
            }
        }
        zone := TouchArea { mouse-cursor: pointer; clicked => { root.clic(); } }
    }

    component BoutonIcone {
        in property <styled-text> infobulle;
        in property <int> icone; // 0 pause, 1 lecture, 2 arrêt
        callback clic();
        width: 36px;
        height: 28px;
        Rectangle {
            border-radius: 3px;
            border-width: 1px;
            border-color: Theme.ligne;
            background: zone.has-hover ? #ffffff1a : #00000040;
            if root.icone == 0: Rectangle {
                width: 10px;
                height: 12px;
                Rectangle { x: 0; width: 4px; background: Theme.texte; }
                Rectangle { x: 6px; width: 4px; background: Theme.texte; }
            }
            if root.icone == 1: Path {
                width: 12px;
                height: 12px;
                commands: "M 0 0 L 10 6 L 0 12 Z";
                fill: Theme.texte;
            }
            if root.icone == 2: Rectangle { width: 10px; height: 10px; background: Theme.texte; }
        }
        zone := TouchArea { mouse-cursor: pointer; clicked => { root.clic(); } }
        Tooltip { text: root.infobulle; }
    }

    component Curseur {
        in-out property <float> valeur;
        height: 18px;
        horizontal-stretch: 0;
        width: 92px;
        Rectangle {
            y: (parent.height - self.height) / 2;
            height: 4px;
            border-radius: 2px;
            background: Theme.ligne;
        }
        Rectangle {
            x: 0;
            y: (parent.height - self.height) / 2;
            width: parent.width * root.valeur;
            height: 4px;
            border-radius: 2px;
            background: Theme.accent;
        }
        Rectangle {
            x: (parent.width - self.width) * root.valeur;
            y: (parent.height - self.height) / 2;
            width: 14px;
            height: 14px;
            border-radius: 7px;
            background: Theme.texte;
        }
        zone := TouchArea {
            function suivre() {
                root.valeur = round(clamp(self.mouse-x / self.width, 0, 1) * 20) / 20;
            }
            pointer-event(e) => { if e.kind == PointerEventKind.down { self.suivre(); } }
            moved => { if self.pressed { self.suivre(); } }
        }
    }

    component Tailles {
        callback moins();
        callback plus();
        HorizontalLayout {
            spacing: 6px;
            Bouton { texte: "A−"; clic => { root.moins(); } }
            Bouton { texte: "A+"; clic => { root.plus(); } }
        }
    }

    component ZoneLive {
        in property <styled-text> definitif;
        in property <styled-text> provisoire;
        in property <color> couleur;
        in property <float> opacite-definitif;
        in property <float> opacite-provisoire;
        in property <length> taille;
        VerticalLayout {
            y: parent.height - self.preferred-height;
            width: parent.width;
            StyledText {
                text: root.definitif;
                default-color: root.couleur.with-alpha(root.opacite-definitif);
                default-font-size: root.taille;
            }
            StyledText {
                text: root.provisoire;
                default-color: root.couleur.with-alpha(root.opacite-provisoire);
                default-font-size: root.taille;
            }
        }
    }

    export component Essai inherits Window {
        in property <styled-text> definitif;
        in property <styled-text> definitif-ombre;
        in property <styled-text> provisoire;
        in property <bool> parle: false;
        in property <[Ligne]> historique;
        in property <string> mesures;
        in property <string> horloge: "00:00:00";
        in property <string> langue: "EN";
        in property <float> niveau: 0.5;
        in property <bool> deplie: false;
        in property <float> position-auto;
        in-out property <bool> au-direct: true;
        in-out property <bool> horodatage: false;
        property <bool> en-pause: false;
        property <int> longueur-sortie: 0;
        property <int> lecture: clamp(
            floor(-liste.content-y / max(1px, liste.content-height) * historique.length),
            0,
            max(0, historique.length - 1));
        property <float> fond: 0.75;
        property <length> taille-live: 22px;
        property <length> taille-histo: 19px;
        callback deplier();
        callback quitter();
        callback deplacer();
        callback horodatage-change();

        title: "essai-slint";
        no-frame: true;
        background: transparent;
        resize-border-width: 6px;
        default-font-family: "Atkinson Hyperlegible";
        preferred-width: 540px;
        preferred-height: 232px;
        min-width: 320px;
        min-height: 200px;

        function coller() {
            liste.content-y = -max(0px, liste.content-height - liste.visible-height);
        }

        function evaluer() {
            let bas = liste.content-height - liste.visible-height;
            let colle = -liste.content-y >= bas - 8px;
            if root.au-direct && !colle {
                root.longueur-sortie = historique.length;
            }
            root.au-direct = colle;
        }

        changed position-auto => {
            liste.content-y = -position-auto * (liste.content-height - liste.visible-height);
        }

        Rectangle {
            background: rgba(8, 12, 15, root.fond);
            border-width: 1px;
            border-color: Theme.ligne;
            border-radius: 8px;
            clip: true;

            VerticalLayout {
                // Pas de WindowMoveArea : le déplacement part dès l'appui, sans seuil de glissement.
                TouchArea {
                    height: 32px;
                    mouse-cursor: move;
                    pointer-event(e) => {
                        if e.kind == PointerEventKind.down && e.button == PointerEventButton.left {
                            root.deplacer();
                        }
                    }
                    HorizontalLayout {
                        padding-left: 12px;
                        padding-right: 12px;
                        spacing: 8px;
                        alignment: stretch;
                        Text {
                            text: "⠿";
                            color: Theme.attenue;
                            font-family: Theme.mono;
                            font-size: 13px;
                            vertical-alignment: center;
                        }
                        Text {
                            text: root.horloge;
                            color: Theme.attenue;
                            font-family: Theme.mono;
                            font-size: 13px;
                            vertical-alignment: center;
                        }
                        Rectangle {
                            width: 44px;
                            Rectangle {
                                y: (parent.height - self.height) / 2;
                                height: 6px;
                                border-radius: 3px;
                                background: Theme.ligne;
                            }
                            Rectangle {
                                x: 0;
                                y: (parent.height - self.height) / 2;
                                width: parent.width * root.niveau;
                                height: 6px;
                                border-radius: 3px;
                                background: Theme.accent;
                            }
                        }
                        Rectangle {
                            y: (parent.height - self.height) / 2;
                            width: puce.preferred-width + 12px;
                            height: 20px;
                            border-width: 1px;
                            border-radius: 3px;
                            border-color: Theme.attenue;
                            puce := Text {
                                text: root.langue;
                                color: Theme.attenue;
                                font-family: Theme.mono;
                                font-size: 12px;
                                font-weight: 600;
                            }
                        }
                        Text {
                            text: "Son système · texte brut + .srt";
                            color: Theme.attenue;
                            font-size: 13px;
                            vertical-alignment: center;
                            horizontal-alignment: right;
                            horizontal-stretch: 1;
                            overflow: elide;
                        }
                    }
                }
                Rectangle { height: 1px; background: Theme.ligne; }

                if root.deplie: HorizontalLayout {
                    padding-left: 12px;
                    padding-right: 12px;
                    padding-top: 6px;
                    height: 30px;
                    Text {
                        text: "HISTORIQUE";
                        color: Theme.accent;
                        font-size: 11px;
                        font-weight: 700;
                        letter-spacing: 0.5px;
                        vertical-alignment: center;
                        horizontal-stretch: 1;
                    }
                    Tailles {
                        moins => { root.taille-histo = max(11px, root.taille-histo * 0.87); }
                        plus => { root.taille-histo = min(48px, root.taille-histo * 1.15); }
                    }
                }
                Rectangle {
                    visible: root.deplie;
                    vertical-stretch: root.deplie ? 1 : 0;
                    max-height: root.deplie ? 100000px : 0px;

                    liste := ListView {
                        x: 12px;
                        width: parent.width - 16px;
                        height: parent.height;
                        scrolled => { root.evaluer(); }
                        changed content-height => { if root.au-direct { root.coller(); } }
                        changed visible-height => { if root.au-direct { root.coller(); } }
                        for ligne in root.historique: VerticalLayout {
                            padding-bottom: root.taille-histo * 0.5;
                            if ligne.l != "": Text {
                                text: ligne.l;
                                color: Theme.accent;
                                font-size: root.taille-histo * 0.62;
                                font-weight: 700;
                                height: self.preferred-height + root.taille-histo * 0.5;
                                vertical-alignment: bottom;
                            }
                            HorizontalLayout {
                                spacing: 6px;
                                if root.horodatage: Text {
                                    text: ligne.h;
                                    color: Theme.attenue;
                                    font-family: Theme.mono;
                                    font-size: root.taille-histo * 0.62;
                                    vertical-alignment: top;
                                    horizontal-stretch: 0;
                                    y: root.taille-histo * 0.4;
                                }
                                Text {
                                    text: ligne.t;
                                    color: Theme.texte;
                                    wrap: word-wrap;
                                    font-size: root.taille-histo;
                                    horizontal-stretch: 1;
                                }
                            }
                        }
                    }
                    Rectangle {
                        x: 0;
                        width: 3px;
                        background: root.au-direct ? transparent : Theme.alerte;
                    }
                    if !root.au-direct: Rectangle {
                        width: pastille.preferred-width + 28px;
                        height: pastille.preferred-height + 12px;
                        x: (parent.width - self.width) / 2;
                        y: parent.height - self.height - 8px;
                        background: Theme.alerte;
                        border-radius: self.height / 2;
                        pastille := Text {
                            text: "Vous lisez " + historique[lecture].h
                                + " · Revenir au direct ("
                                + (historique.length - longueur-sortie) + ")";
                            color: #2a1c00;
                            font-size: 14px;
                            font-weight: 700;
                        }
                        TouchArea {
                            mouse-cursor: pointer;
                            clicked => { root.au-direct = true; root.coller(); }
                        }
                    }
                }
                if root.deplie: Rectangle { height: 1px; background: Theme.ligne; }

                VerticalLayout {
                    padding-left: 12px;
                    padding-right: 12px;
                    padding-top: 6px;
                    HorizontalLayout {
                        height: 24px;
                        Text {
                            text: root.en-pause ? "EN PAUSE" : "● DIRECT";
                            color: root.en-pause ? Theme.alerte : Theme.accent;
                            font-size: 11px;
                            font-weight: 700;
                            letter-spacing: 0.5px;
                            vertical-alignment: center;
                            horizontal-stretch: 1;
                        }
                        Tailles {
                            moins => { root.taille-live = max(14px, root.taille-live * 0.87); }
                            plus => { root.taille-live = min(48px, root.taille-live * 1.15); }
                        }
                    }
                    // Hauteur fixe et texte calé en bas : ce sont les mots les plus anciens qui
                    // sortent par le haut, jamais les derniers.
                    Rectangle {
                        height: root.taille-live * 3.9;
                        clip: true;
                        ZoneLive {
                            x: 1px;
                            y: 1px;
                            width: parent.width;
                            height: parent.height;
                            definitif: root.definitif-ombre;
                            provisoire: root.provisoire;
                            couleur: #000000;
                            opacite-definitif: 0.8;
                            opacite-provisoire: 0.8;
                            taille: root.taille-live;
                        }
                        ZoneLive {
                            width: parent.width;
                            height: parent.height;
                            definitif: root.definitif;
                            provisoire: root.provisoire;
                            couleur: Theme.texte;
                            opacite-definitif: root.parle ? 0.72 : 1.0;
                            opacite-provisoire: 0.78;
                            taille: root.taille-live;
                        }
                    }
                }
                // Ligne d'alerte réservée : les mesures y logent le temps de l'essai.
                Text {
                    height: 20px;
                    x: 12px;
                    text: root.mesures;
                    color: Theme.attenue;
                    font-family: Theme.mono;
                    font-size: 10px;
                    vertical-alignment: center;
                }
                HorizontalLayout {
                    padding-left: 12px;
                    padding-right: 12px;
                    padding-top: 2px;
                    padding-bottom: 10px;
                    spacing: 8px;
                    height: 40px;
                    Bouton { texte: "Historique"; actif: root.deplie; taille: 13px; clic => { root.deplier(); } }
                    Bouton {
                        texte: "Horodatage";
                        actif: root.horodatage;
                        taille: 13px;
                        clic => { root.horodatage = !root.horodatage; root.horodatage-change(); }
                    }
                    Text { text: "Fond"; color: Theme.texte; font-size: 13px; vertical-alignment: center; }
                    VerticalLayout { alignment: center; horizontal-stretch: 0; Curseur { valeur <=> root.fond; } }
                    Text {
                        text: round(root.fond * 100) + " %";
                        color: Theme.texte;
                        font-family: Theme.mono;
                        font-size: 12px;
                        vertical-alignment: center;
                        horizontal-stretch: 1;
                    }
                    BoutonIcone {
                        infobulle: root.en-pause ? @markdown("Reprendre") : @markdown("Pause");
                        icone: root.en-pause ? 1 : 0;
                        clic => { root.en-pause = !root.en-pause; }
                    }
                    BoutonIcone {
                        infobulle: @markdown("Arrêter");
                        icone: 2;
                        clic => { root.quitter(); }
                    }
                }
            }
        }
    }
}

/// `StyledText` n'accepte que du Markdown, sans fond par span : le mot modifié est rendu en
/// couleur soulignée.
fn md_definitif(flux: &Flux, horodatage: bool, ombre: bool) -> slint::StyledText {
    let mut md = String::new();
    if horodatage {
        // L'ombre garde l'heure : sans elle, ses lignes se replieraient autrement que le texte.
        let heure = flux.dernier.horodatage();
        md += &if ombre { format!("{heure} ") } else { format!("<font color=\"#9fb0bc\">{heure}</font> ") };
    }
    for mot in &flux.dernier.mots {
        if mot.modifie && !ombre {
            md += &format!("<font color=\"#ffe08a\"><u>{}</u></font> ", mot.texte);
        } else {
            md += &format!("{} ", mot.texte);
        }
    }
    slint::StyledText::from_markdown(&md).expect("Markdown du définitif")
}

fn md_provisoire(flux: &Flux) -> slint::StyledText {
    let md = if flux.provisoire.is_empty() {
        String::new()
    } else {
        format!("*{}*", flux.provisoire.join(" "))
    };
    slint::StyledText::from_markdown(&md).expect("Markdown du provisoire")
}

fn afficher_live(app: &Essai, flux: &Flux) {
    let horodatage = app.get_horodatage();
    app.set_definitif(md_definitif(flux, horodatage, false));
    app.set_definitif_ombre(md_definitif(flux, horodatage, true));
    app.set_provisoire(md_provisoire(flux));
    app.set_parle(!flux.provisoire.is_empty());
    app.set_langue(flux.courante.into());
    app.set_niveau(0.45 + 0.4 * (flux.version as f32 * 0.9).sin().abs());
}

/// L'en-tête de langue n'apparaît qu'au changement de Langue parlée.
fn lignes(historique: &[commun::Enonce], mut precedente: &str) -> Vec<Ligne> {
    historique
        .iter()
        .map(|e| {
            let l = if e.langue != precedente { nom_langue(e.langue) } else { "" };
            precedente = e.langue;
            Ligne { h: e.horodatage().into(), t: e.texte().into(), l: l.into() }
        })
        .collect()
}

fn nom_langue(code: &str) -> &'static str {
    match code {
        "DE" => "Deutsch",
        "FR" => "Français",
        _ => "English",
    }
}

fn deplier(app: &Essai) {
    let deplie = !app.get_deplie();
    app.set_deplie(deplie);
    let hauteur = if deplie { HAUTEUR_HISTORIQUE } else { HAUTEUR_COMPACTE };
    let echelle = app.window().scale_factor();
    let actuelle = app.window().size().height as f32 / echelle;
    let largeur = app.window().size().width as f32 / echelle;
    let position = app.window().with_winit_window(|w| w.outer_position().ok()).flatten();
    app.window().set_size(slint::LogicalSize::new(largeur, hauteur));
    match position {
        Some(p) => {
            let y = p.y - ((hauteur - actuelle) * echelle) as i32;
            app.window().set_position(slint::PhysicalPosition::new(p.x, y));
            commun::note("deplier", "position lue, déplacement vers le haut demandé");
        }
        None => commun::note("deplier", "position illisible : le bas ne peut pas rester fixe"),
    }
}

fn main() -> Result<(), slint::PlatformError> {
    commun::initialiser("slint");
    commun::note("rendu", &std::env::var("SLINT_BACKEND").unwrap_or("defaut".into()));
    let app = Essai::new()?;
    let faible = app.as_weak();

    let flux = commun::demarrer_flux(move || {
        let faible = faible.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(app) = faible.upgrade() else { return };
            let flux = FLUX.get().unwrap().lock().unwrap();
            afficher_live(&app, &flux);
            let modele = app.get_historique();
            if let Some(vec) = modele.as_any().downcast_ref::<VecModel<Ligne>>() {
                let n = vec.row_count();
                if n < flux.historique.len() {
                    let avant = flux.historique[n - 1].langue;
                    for l in lignes(&flux.historique[n..], avant) {
                        vec.push(l);
                    }
                }
            }
        });
    });
    let contenu = lignes(&flux.lock().unwrap().historique, "");
    afficher_live(&app, &flux.lock().unwrap());
    app.set_historique(ModelRc::from(Rc::new(VecModel::from(contenu))));
    FLUX.set(flux).ok();

    if commun::mode_auto() {
        app.set_au_direct(false);
    }
    if commun::mode_auto() || std::env::args().any(|a| a == "--deplie") {
        app.set_deplie(true);
        app.window().set_size(slint::LogicalSize::new(640.0, HAUTEUR_HISTORIQUE));
    }
    let notifier = app.window().set_rendering_notifier(|etat, _| {
        if matches!(etat, slint::RenderingState::AfterRendering) {
            commun::sonde().lock().unwrap().frame();
        }
    });
    if let Err(e) = notifier {
        commun::note("sonde", &format!("notifier indisponible : {e:?}"));
    }

    let copie = app.as_weak();
    app.on_deplier(move || deplier(&copie.unwrap()));
    app.on_quitter(|| commun::quitter());
    let copie = app.as_weak();
    app.on_horodatage_change(move || {
        afficher_live(&copie.unwrap(), &FLUX.get().unwrap().lock().unwrap());
    });
    let copie = app.as_weak();
    app.on_deplacer(move || {
        // Le compositeur prend le pointeur et Slint ne verra jamais le relâchement : sans ces
        // événements, la TouchArea garderait la souris et capterait tous les clics suivants.
        let copie = copie.clone();
        Timer::single_shot(Duration::ZERO, move || {
            let app = copie.unwrap();
            let fenetre = app.window();
            fenetre.with_winit_window(|w| w.drag_window());
            fenetre.dispatch_event(WindowEvent::PointerReleased {
                position: slint::LogicalPosition::default(),
                button: PointerEventButton::Left,
            });
            fenetre.dispatch_event(WindowEvent::PointerExited);
        });
    });

    let copie = app.as_weak();
    let debut = std::time::Instant::now();
    let releve = Timer::default();
    releve.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        let app = copie.unwrap();
        app.set_mesures(commun::ligne_mesures().into());
        let s = debut.elapsed().as_secs();
        app.set_horloge(format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60).into());
    });
    let copie = app.as_weak();
    let defilement = Timer::default();
    if commun::mode_auto() {
        defilement.start(TimerMode::Repeated, Duration::from_millis(16), move || {
            if let Some(f) = commun::defilement_auto() {
                copie.unwrap().set_position_auto(f);
            }
        });
    }
    app.run()
}
