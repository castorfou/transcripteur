//! PROTOTYPE jetable : essai de Slint pour la forme de l'affichage live.

use commun::Flux;
use slint::winit_030::WinitWindowAccessor;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const HAUTEUR_COMPACTE: f32 = 210.0;
const HAUTEUR_HISTORIQUE: f32 = 420.0;

static FLUX: OnceLock<Arc<Mutex<Flux>>> = OnceLock::new();

slint::slint! {
    import { Button, ListView, Slider } from "std-widgets.slint";

    export component Essai inherits Window {
        in property <styled-text> live;
        in property <[string]> historique;
        in property <string> mesures;
        in property <bool> deplie: false;
        in property <float> position-auto;
        property <float> fond: 0.6;
        property <length> taille-live: 22px;
        property <length> taille-histo: 15px;
        callback deplier();
        callback quitter();

        title: "essai-slint";
        no-frame: true;
        background: transparent;
        resize-border-width: 6px;
        preferred-width: 640px;
        preferred-height: 210px;
        min-width: 320px;
        min-height: 150px;

        changed position-auto => {
            liste.content-y = -position-auto * (liste.content-height - liste.visible-height);
        }

        Rectangle {
            background: rgba(0, 0, 0, root.fond);
            border-width: 1px;
            border-color: #3c3c3c;

            VerticalLayout {
                padding: 10px;
                spacing: 6px;

                WindowMoveArea {
                    height: 20px;
                    HorizontalLayout {
                        spacing: 10px;
                        Text { text: "Slint"; color: white; font-weight: 700; }
                        Text { text: root.mesures; color: white; font-size: 11px; }
                    }
                }
                HorizontalLayout {
                    spacing: 10px;
                    height: 32px;
                    Text { text: "Fond"; color: white; vertical-alignment: center; horizontal-stretch: 0; }
                    Slider {
                        horizontal-stretch: 1;
                        minimum: 0; maximum: 1; value: root.fond;
                        changed(v) => { root.fond = v; }
                    }
                    Button { text: root.deplie ? "Replier" : "Déplier"; clicked => { root.deplier(); } }
                    Button { text: "Fermer"; clicked => { root.quitter(); } }
                }
                if root.deplie: HorizontalLayout {
                    spacing: 6px;
                    height: 28px;
                    Text { text: "Historique"; color: white; font-size: 11px; vertical-alignment: center; horizontal-stretch: 1; }
                    Button { text: "A−"; horizontal-stretch: 0; width: 44px; clicked => { root.taille-histo = max(10px, root.taille-histo - 2px); } }
                    Button { text: "A+"; horizontal-stretch: 0; width: 44px; clicked => { root.taille-histo = min(48px, root.taille-histo + 2px); } }
                }
                liste := ListView {
                    visible: root.deplie;
                    vertical-stretch: root.deplie ? 1 : 0;
                    max-height: root.deplie ? 100000px : 0px;
                    for ligne in root.historique: VerticalLayout {
                        padding-bottom: 4px;
                        Text {
                            text: ligne;
                            color: #d3d3d3;
                            wrap: word-wrap;
                            font-size: root.taille-histo;
                        }
                    }
                }
                HorizontalLayout {
                    spacing: 6px;
                    height: 28px;
                    Text { text: "Live"; color: white; font-size: 11px; vertical-alignment: center; horizontal-stretch: 1; }
                    Button { text: "A−"; horizontal-stretch: 0; width: 44px; clicked => { root.taille-live = max(10px, root.taille-live - 2px); } }
                    Button { text: "A+"; horizontal-stretch: 0; width: 44px; clicked => { root.taille-live = min(48px, root.taille-live + 2px); } }
                }
                StyledText {
                    text: root.live;
                    default-color: white;
                    default-font-size: root.taille-live;
                    vertical-stretch: root.deplie ? 0 : 1;
                }
            }
        }
    }
}

/// `StyledText` n'accepte que du Markdown, sans fond par span : le mot modifié est
/// rendu en couleur soulignée, faute de pouvoir le surligner comme dans la forme validée.
fn markdown_live(flux: &Flux) -> slint::StyledText {
    let mut md = String::new();
    for mot in &flux.dernier.mots {
        if mot.modifie {
            md += &format!("<font color=\"gold\"><u>{}</u></font> ", mot.texte);
        } else {
            md += &format!("{} ", mot.texte);
        }
    }
    if !flux.provisoire.is_empty() {
        md += &format!("<font color=\"#aaaaaa\">*{}*</font>", flux.provisoire.join(" "));
    }
    slint::StyledText::from_markdown(&md).expect("Markdown du live")
}

fn ligne(e: &commun::Enonce) -> SharedString {
    format!("{}  {}", e.horodatage(), e.texte()).into()
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
            app.set_live(markdown_live(&flux));
            let modele = app.get_historique();
            if let Some(vec) = modele.as_any().downcast_ref::<VecModel<SharedString>>() {
                for e in &flux.historique[vec.row_count()..] {
                    vec.push(ligne(e));
                }
            }
        });
    });
    let lignes: Vec<_> = flux.lock().unwrap().historique.iter().map(ligne).collect();
    app.set_live(markdown_live(&flux.lock().unwrap()));
    app.set_historique(ModelRc::from(Rc::new(VecModel::from(lignes))));
    FLUX.set(flux).ok();

    if commun::mode_auto() {
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
    let releve = Timer::default();
    releve.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        copie.unwrap().set_mesures(commun::ligne_mesures().into());
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
