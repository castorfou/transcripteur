//! PROTOTYPE jetable : essai d'iced pour la forme de l'affichage live.

use commun::Flux;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;
use iced::widget::scrollable::RelativeOffset;
use iced::widget::text::Span;
use iced::widget::{
    Column, button, column, container, lazy, mouse_area, operation, row, scrollable, slider, space,
    span, stack, text,
};
use iced::window::{self, Direction};
use iced::{
    Background, Color, Element, Font, Length, Point, Rectangle, Size, Subscription, Task, Theme,
    font, mouse, theme,
};
use std::sync::{Arc, Mutex};

const HAUTEUR_COMPACTE: f32 = 210.0;
const HAUTEUR_HISTORIQUE: f32 = 420.0;
const BORD: f32 = 6.0;
/// cosmic-text fixe `sans-serif` sur « Open Sans » et panique si elle manque : on embarque.
const POLICE: Font = Font::with_name("DejaVu Sans");

static RECEPTEUR: Mutex<Option<mpsc::UnboundedReceiver<()>>> = Mutex::new(None);

struct Essai {
    flux: Arc<Mutex<Flux>>,
    fond: f32,
    taille_live: f32,
    taille_histo: f32,
    deplie: bool,
    id: Option<window::Id>,
    longueur_histo: usize,
}

#[derive(Debug, Clone)]
enum Message {
    Ouvert(Option<window::Id>),
    Tick,
    Frame,
    Fond(f32),
    TailleLive(f32),
    TailleHisto(f32),
    Deplier,
    Position(Option<Point>),
    Deplacer,
    Redimensionner(Direction),
    Quitter,
}

/// Point de mesure : `draw` n'est appelé que lorsqu'une frame est vraiment dessinée.
struct Sonde;

impl<M> Widget<M, Theme, iced::Renderer> for Sonde {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(1.0), Length::Fixed(1.0))
    }

    fn layout(
        &mut self,
        _tree: &mut widget::Tree,
        _renderer: &iced::Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(1.0, 1.0)) // taille nulle : iced ne l'aurait jamais dessinée
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        _renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        commun::sonde().lock().unwrap().frame();
    }
}

fn boot() -> (Essai, Task<Message>) {
    commun::initialiser("iced");
    let (envoi, reception) = mpsc::unbounded();
    *RECEPTEUR.lock().unwrap() = Some(reception);
    let flux = commun::demarrer_flux(move || {
        let _ = envoi.unbounded_send(());
    });
    let longueur_histo = flux.lock().unwrap().historique.len();
    let etat = Essai {
        flux,
        fond: 0.6,
        taille_live: 22.0,
        taille_histo: 15.0,
        deplie: commun::mode_auto(),
        id: None,
        longueur_histo,
    };
    (etat, window::latest().map(Message::Ouvert))
}

fn update(e: &mut Essai, message: Message) -> Task<Message> {
    match message {
        Message::Ouvert(id) => e.id = id,
        Message::Tick => e.longueur_histo = e.flux.lock().unwrap().historique.len(),
        Message::Frame => {
            if let Some(f) = commun::defilement_auto() {
                let cible = RelativeOffset { x: None, y: Some(f) };
                return operation::snap_to("historique", cible);
            }
        }
        Message::Fond(f) => e.fond = f,
        Message::TailleLive(t) => e.taille_live = t.clamp(10.0, 48.0),
        Message::TailleHisto(t) => e.taille_histo = t.clamp(10.0, 48.0),
        Message::Deplier => {
            e.deplie = !e.deplie;
            if let Some(id) = e.id {
                return window::position(id).map(Message::Position);
            }
        }
        Message::Position(position) => {
            let Some(id) = e.id else { return Task::none() };
            let hauteur = if e.deplie { HAUTEUR_HISTORIQUE } else { HAUTEUR_COMPACTE };
            let delta = HAUTEUR_HISTORIQUE - HAUTEUR_COMPACTE;
            let delta = if e.deplie { delta } else { -delta };
            let taille = window::resize(id, Size::new(640.0, hauteur));
            return match position {
                Some(p) => {
                    commun::note("deplier", "position lue, déplacement vers le haut demandé");
                    Task::batch([taille, window::move_to(id, Point::new(p.x, p.y - delta))])
                }
                None => {
                    commun::note("deplier", "position illisible : le bas ne peut pas rester fixe");
                    taille
                }
            };
        }
        Message::Deplacer => {
            if let Some(id) = e.id {
                return window::drag(id);
            }
        }
        Message::Redimensionner(direction) => {
            if let Some(id) = e.id {
                return window::drag_resize(id, direction);
            }
        }
        Message::Quitter => commun::quitter(),
    }
    Task::none()
}

fn zone_live<'a>(e: &Essai, flux: &Flux) -> Element<'a, Message> {
    let italique = Font { style: font::Style::Oblique, ..POLICE };
    let mut spans: Vec<Span<'a, Message>> = Vec::new();
    for mot in &flux.dernier.mots {
        let s = span(format!("{} ", mot.texte));
        spans.push(if mot.modifie {
            s.background(Background::Color(Color::from_rgb8(120, 90, 0)))
        } else {
            s
        });
    }
    spans.push(span(flux.provisoire.join(" ")).font(italique).color(Color::from_rgb8(170, 170, 170)));
    iced::widget::rich_text(spans).size(e.taille_live).into()
}

/// Colonne complète : iced n'a pas de liste virtualisée. `lazy` évite de reconstruire
/// les 1500 widgets à chaque tick, seule la longueur et la taille les invalident.
fn zone_historique<'a>(e: &Essai) -> Element<'a, Message> {
    let flux = e.flux.clone();
    let taille = e.taille_histo;
    let liste = lazy((e.longueur_histo, taille as u32), move |_| {
        let flux = flux.lock().unwrap();
        let lignes = flux.historique.iter().map(|enonce| {
            text(format!("{}  {}", enonce.horodatage(), enonce.texte())).size(taille).into()
        });
        Column::with_children(lignes).spacing(4)
    });
    scrollable(liste).id("historique").height(Length::Fill).width(Length::Fill).into()
}

fn bord<'a>(direction: Direction, l: Length, h: Length) -> Element<'a, Message> {
    let curseur = match direction {
        Direction::North | Direction::South => mouse::Interaction::ResizingVertically,
        Direction::East | Direction::West => mouse::Interaction::ResizingHorizontally,
        Direction::NorthWest | Direction::SouthEast => mouse::Interaction::ResizingDiagonallyDown,
        Direction::NorthEast | Direction::SouthWest => mouse::Interaction::ResizingDiagonallyUp,
    };
    mouse_area(space().width(l).height(h))
        .interaction(curseur)
        .on_press(Message::Redimensionner(direction))
        .into()
}

fn bords<'a>() -> Element<'a, Message> {
    let (b, f) = (Length::Fixed(BORD), Length::Fill);
    column![
        row![
            bord(Direction::NorthWest, b, b),
            bord(Direction::North, f, b),
            bord(Direction::NorthEast, b, b)
        ],
        row![
            bord(Direction::West, b, f),
            space().width(f).height(f),
            bord(Direction::East, b, f)
        ]
        .height(f),
        row![
            bord(Direction::SouthWest, b, b),
            bord(Direction::South, f, b),
            bord(Direction::SouthEast, b, b)
        ],
    ]
    .into()
}

fn boutons_taille<'a>(taille: f32, m: fn(f32) -> Message) -> Element<'a, Message> {
    row![
        button(text("A−").size(12)).padding(2).on_press(m(taille - 2.0)),
        button(text("A+").size(12)).padding(2).on_press(m(taille + 2.0)),
    ]
    .spacing(4)
    .into()
}

fn view(e: &Essai) -> Element<'_, Message> {
    let flux = e.flux.lock().unwrap();
    let entete = mouse_area(
        container(row![
            text("iced").size(14),
            space().width(10),
            text(commun::ligne_mesures()).size(11)
        ])
        .width(Length::Fill),
    )
    .interaction(mouse::Interaction::Grab)
    .on_press(Message::Deplacer);
    let commandes = row![
        text("Fond").size(12),
        slider(0.0..=1.0, e.fond, Message::Fond).step(0.01_f32).width(160),
        button(text(if e.deplie { "Replier" } else { "Déplier" }).size(12))
            .on_press(Message::Deplier),
        button(text("Fermer").size(12)).on_press(Message::Quitter),
    ]
    .spacing(10);

    let mut contenu = column![
        Element::new(Sonde),
        entete,
        commandes
    ]
    .spacing(8);
    if e.deplie {
        contenu = contenu
            .push(row![text("Historique").size(11), space().width(8), boutons_taille(e.taille_histo, Message::TailleHisto)])
            .push(zone_historique(e));
    }
    contenu = contenu
        .push(row![text("Live").size(11), space().width(8), boutons_taille(e.taille_live, Message::TailleLive)])
        .push(zone_live(e, &flux));

    let fond = e.fond;
    let cadre = container(contenu)
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, fond))),
            text_color: Some(Color::WHITE),
            ..Default::default()
        });
    stack![cadre, bords()].into()
}

fn subscription(_e: &Essai) -> Subscription<Message> {
    fn ticks() -> impl iced::futures::Stream<Item = Message> {
        let reception = RECEPTEUR.lock().unwrap().take().expect("une seule souscription");
        reception.map(|()| Message::Tick)
    }
    let flux = Subscription::run(ticks);
    if commun::defilement_auto().is_some() {
        Subscription::batch([flux, window::frames().map(|_| Message::Frame)])
    } else {
        flux
    }
}

fn main() -> iced::Result {
    commun::note("rendu", &std::env::var("ICED_BACKEND").unwrap_or("wgpu (défaut)".into()));
    let hauteur = if commun::mode_auto() { HAUTEUR_HISTORIQUE } else { HAUTEUR_COMPACTE };
    iced::application(boot, update, view)
        .font(include_bytes!("../polices/DejaVuSans.ttf").as_slice())
        .font(include_bytes!("../polices/DejaVuSans-Oblique.ttf").as_slice())
        .default_font(POLICE)
        .title(|_: &Essai| "essai-iced".to_owned())
        .subscription(subscription)
        .theme(|_: &Essai| Theme::Dark)
        .style(|_, _| theme::Style { background_color: Color::TRANSPARENT, text_color: Color::WHITE })
        .window(window::Settings {
            size: Size::new(640.0, hauteur),
            min_size: Some(Size::new(320.0, 150.0)),
            decorations: false,
            transparent: true,
            platform_specific: window::settings::PlatformSpecific {
                application_id: "essai-iced".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
