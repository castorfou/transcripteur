//! PROTOTYPE jetable : essai d'egui/eframe pour la forme de l'affichage live.

use commun::{Enonce, Flux};
use eframe::egui::{self, Color32, FontId, RichText, Sense, Stroke, TextFormat, Vec2};
use egui::text::LayoutJob;
use egui::{ResizeDirection, ViewportCommand};
use std::sync::{Arc, Mutex};

const HAUTEUR_COMPACTE: f32 = 210.0;
const HAUTEUR_HISTORIQUE: f32 = 420.0;
const BORD: f32 = 6.0;

struct Essai {
    flux: Arc<Mutex<Flux>>,
    fond: f32,
    taille_live: f32,
    taille_histo: f32,
    deplie: bool,
    /// Hauteurs cumulées de l'historique, à refaire si la largeur ou la taille change.
    hauteurs: Vec<f32>,
    cle_hauteurs: (u32, u32, usize),
}

impl Essai {
    fn zone_live(&self, ui: &mut egui::Ui, flux: &Flux) {
        let police = FontId::proportional(self.taille_live);
        let mut job = LayoutJob::default();
        job.wrap.max_width = ui.available_width();
        for mot in &flux.dernier.mots {
            let mut format = TextFormat::simple(police.clone(), Color32::WHITE);
            if mot.modifie {
                format.background = Color32::from_rgb(120, 90, 0);
            }
            job.append(&format!("{} ", mot.texte), 0.0, format);
        }
        let mut provisoire = TextFormat::simple(police, Color32::from_gray(170));
        provisoire.italics = true;
        job.append(&flux.provisoire.join(" "), 0.0, provisoire);
        ui.label(job);
    }

    /// `show_rows` exige des lignes de même hauteur, or le texte est replié :
    /// on cumule nous-mêmes les hauteurs et on ne dessine que ce qui est visible.
    fn zone_historique(&mut self, ui: &mut egui::Ui, flux: &Flux) {
        let police = FontId::proportional(self.taille_histo);
        let largeur = ui.available_width() - 14.0;
        let cle = (largeur as u32, self.taille_histo as u32, flux.historique.len());
        let fonts_layout = |ui: &egui::Ui, e: &Enonce| {
            ui.painter().layout(
                format!("{}  {}", e.horodatage(), e.texte()),
                police.clone(),
                Color32::LIGHT_GRAY,
                largeur,
            )
        };
        if (cle.0, cle.1) != (self.cle_hauteurs.0, self.cle_hauteurs.1) {
            self.hauteurs.clear();
            let mut y = 0.0;
            for e in &flux.historique {
                y += fonts_layout(ui, e).size().y + 4.0;
                self.hauteurs.push(y);
            }
        } else if cle.2 != self.cle_hauteurs.2 {
            let mut y = self.hauteurs.last().copied().unwrap_or(0.0);
            for e in &flux.historique[self.hauteurs.len()..] {
                y += fonts_layout(ui, e).size().y + 4.0;
                self.hauteurs.push(y);
            }
        }
        self.cle_hauteurs = cle;
        let total = self.hauteurs.last().copied().unwrap_or(0.0);

        let mut zone = egui::ScrollArea::vertical().auto_shrink(false);
        if let Some(f) = commun::defilement_auto() {
            let visible = ui.available_height();
            zone = zone.vertical_scroll_offset(f * (total - visible).max(0.0));
        }
        zone.show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            let origine = ui.min_rect().min;
            let premier = self.hauteurs.partition_point(|y| *y < viewport.min.y);
            for i in premier..flux.historique.len() {
                let y = if i == 0 { 0.0 } else { self.hauteurs[i - 1] };
                if y > viewport.max.y {
                    break;
                }
                let galley = fonts_layout(ui, &flux.historique[i]);
                ui.painter().galley(origine + Vec2::new(0.0, y), galley, Color32::LIGHT_GRAY);
            }
        });
    }

    fn deplier(&mut self, ctx: &egui::Context) {
        self.deplie = !self.deplie;
        let hauteur = if self.deplie { HAUTEUR_HISTORIQUE } else { HAUTEUR_COMPACTE };
        let (largeur, delta, position) = ctx.input(|i| {
            let v = i.viewport();
            let actuelle = v.inner_rect.map_or(HAUTEUR_COMPACTE, |r| r.height());
            (v.inner_rect.map_or(640.0, |r| r.width()), hauteur - actuelle, v.outer_rect)
        });
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(largeur, hauteur)));
        match position {
            Some(r) => {
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(r.min - Vec2::new(0.0, delta)));
                commun::note("deplier", "position lue, déplacement vers le haut demandé");
            }
            None => commun::note("deplier", "position illisible : le bas ne peut pas rester fixe"),
        }
    }
}

fn poignees_bords(ctx: &egui::Context) {
    let Some(p) = ctx.pointer_hover_pos() else { return };
    let r = ctx.content_rect();
    let (g, d) = (p.x < r.left() + BORD, p.x > r.right() - BORD);
    let (h, b) = (p.y < r.top() + BORD, p.y > r.bottom() - BORD);
    let direction = match (h, b, g, d) {
        (true, _, true, _) => ResizeDirection::NorthWest,
        (true, _, _, true) => ResizeDirection::NorthEast,
        (_, true, true, _) => ResizeDirection::SouthWest,
        (_, true, _, true) => ResizeDirection::SouthEast,
        (true, ..) => ResizeDirection::North,
        (_, true, ..) => ResizeDirection::South,
        (_, _, true, _) => ResizeDirection::West,
        (_, _, _, true) => ResizeDirection::East,
        _ => return,
    };
    ctx.set_cursor_icon(match direction {
        ResizeDirection::North => egui::CursorIcon::ResizeNorth,
        ResizeDirection::South => egui::CursorIcon::ResizeSouth,
        ResizeDirection::East => egui::CursorIcon::ResizeEast,
        ResizeDirection::West => egui::CursorIcon::ResizeWest,
        ResizeDirection::NorthEast => egui::CursorIcon::ResizeNorthEast,
        ResizeDirection::NorthWest => egui::CursorIcon::ResizeNorthWest,
        ResizeDirection::SouthEast => egui::CursorIcon::ResizeSouthEast,
        ResizeDirection::SouthWest => egui::CursorIcon::ResizeSouthWest,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}

fn boutons_taille(ui: &mut egui::Ui, taille: &mut f32) {
    if ui.small_button("A−").clicked() {
        *taille = (*taille - 2.0).max(10.0);
    }
    if ui.small_button("A+").clicked() {
        *taille = (*taille + 2.0).min(48.0);
    }
}

impl eframe::App for Essai {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if commun::defilement_auto().is_some() {
            ctx.request_repaint();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            commun::quitter();
        }
        poignees_bords(&ctx);
        let flux = self.flux.clone();
        let flux = flux.lock().unwrap();
        let alpha = (self.fond * 255.0) as u8;
        let cadre = egui::Frame::NONE
            .fill(Color32::from_black_alpha(alpha))
            .inner_margin(10.0)
            .stroke(Stroke::new(1.0, Color32::from_gray(60)));
        egui::CentralPanel::default().frame(cadre).show_inside(ui, |ui| {
            ui.visuals_mut().override_text_color = Some(Color32::WHITE);
            let entete = ui.horizontal(|ui| {
                ui.label(RichText::new("egui").strong());
                ui.small(commun::ligne_mesures());
            });
            let poignee = ui.interact(entete.response.rect, ui.id().with("poignee"), Sense::drag());
            if poignee.drag_started() {
                ctx.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut self.fond, 0.0..=1.0).text("Fond"));
                if ui.button(if self.deplie { "Replier" } else { "Déplier" }).clicked() {
                    self.deplier(&ctx);
                }
                if ui.button("Fermer").clicked() {
                    commun::quitter();
                }
            });
            ui.separator();
            if self.deplie {
                let hauteur = ui.available_height() - 110.0;
                ui.allocate_ui(Vec2::new(ui.available_width(), hauteur), |ui| {
                    ui.horizontal(|ui| {
                        ui.small("Historique");
                        boutons_taille(ui, &mut self.taille_histo);
                    });
                    self.zone_historique(ui, &flux);
                });
                ui.separator();
            }
            ui.horizontal(|ui| {
                ui.small("Live");
                boutons_taille(ui, &mut self.taille_live);
            });
            self.zone_live(ui, &flux);
        });
        commun::sonde().lock().unwrap().frame();
    }
}

fn main() -> eframe::Result {
    commun::initialiser("egui");
    let deplie = commun::mode_auto();
    let viewport = egui::ViewportBuilder::default()
        .with_app_id("essai-egui")
        .with_title("essai-egui")
        .with_decorations(false)
        .with_transparent(true)
        .with_min_inner_size([320.0, 150.0])
        .with_inner_size([640.0, if deplie { HAUTEUR_HISTORIQUE } else { HAUTEUR_COMPACTE }]);
    let renderer = match std::env::var("EGUI_RENDERER").as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    };
    commun::note("rendu", &format!("{renderer:?}"));
    let options = eframe::NativeOptions { viewport, renderer, ..Default::default() };
    eframe::run_native(
        "essai-egui",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let flux = commun::demarrer_flux(move || ctx.request_repaint());
            Ok(Box::new(Essai {
                flux,
                fond: 0.6,
                taille_live: 22.0,
                taille_histo: 15.0,
                deplie,
                hauteurs: Vec::new(),
                cle_hauteurs: (0, 0, 0),
            }))
        }),
    )
}
