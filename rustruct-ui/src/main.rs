use eframe::egui;
use rustruct_types::SeismicModel;

fn main() -> eframe::Result {
    eframe::run_native(
        "Rustruct",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "noto_sans_jp".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/NotoSansJP-Regular.otf"
                ))),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .push("noto_sans_jp".to_owned());
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(RustructApp::default()))
        }),
    )
}

struct RustructApp {
    model: SeismicModel,
}
impl Default for RustructApp {
    fn default() -> Self {
        Self {
            model: SeismicModel {
                name: "新しいモデル".to_string(),
                story_masses: vec![1.0e3, 1.0e3],
                story_stiffnesses: vec![5.0e5, 5.0e5],
                story_heights: vec![3.5, 3.5],
            },
        }
    }
}

impl eframe::App for RustructApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            let mut to_remove: Option<usize> = None;
            let can_remove = self.model.story_masses.len() > 1;
            ui.label(format!("階数: {}", self.model.story_masses.len()));
            if ui.button("階を追加").clicked() {
                self.model
                    .story_masses
                    .push(self.model.story_masses.last().copied().unwrap_or(3.5));
                self.model.story_stiffnesses.push(
                    self.model
                        .story_stiffnesses
                        .last()
                        .copied()
                        .unwrap_or(5.0e5),
                );
                self.model
                    .story_heights
                    .push(self.model.story_heights.last().copied().unwrap_or(3.5));
            }
            for i in 0..self.model.story_masses.len() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}F", i + 1));
                    ui.add(egui::DragValue::new(&mut self.model.story_heights[i]).speed(0.01));
                    ui.add(egui::DragValue::new(&mut self.model.story_masses[i]).speed(10.0));
                    ui.add(egui::DragValue::new(&mut self.model.story_stiffnesses[i]).speed(10.0));
                    if ui
                        .add_enabled(can_remove, egui::Button::new("remove"))
                        .clicked()
                    {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(index) = to_remove {
                self.model.story_masses.remove(index);
                self.model.story_stiffnesses.remove(index);
                self.model.story_heights.remove(index);
            }
            ui.allocate_space(ui.available_size());
        });
    }
}
