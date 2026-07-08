use eframe::egui;
use rustruct_types::{SeismicModel, Story};
use std::sync::{Arc, Mutex};

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
    save_state: Arc<Mutex<SaveState>>,
}
impl Default for RustructApp {
    fn default() -> Self {
        Self {
            model: SeismicModel {
                name: "新しいモデル".to_string(),
                stories: vec![
                    Story {
                        height: 3.5,
                        mass: 1.0e3,
                        stiffness: 5.0e5,
                    },
                    Story {
                        height: 3.5,
                        mass: 1.0e3,
                        stiffness: 5.0e5,
                    },
                ],
            },
            save_state: Arc::new(Mutex::new(SaveState::Idle)),
        }
    }
}

enum SaveState {
    Idle,
    Saving,
    Success,
    Error(String),
}

impl eframe::App for RustructApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            let mut to_remove: Option<usize> = None;
            let n = self.model.story_count();
            let can_remove = n > 1;
            ui.text_edit_singleline(&mut self.model.name);
            ui.label(format!("階数: {}", n));
            if ui.button("階を追加").clicked() {
                self.model
                    .add_story(self.model.stories.last().copied().unwrap_or(Story {
                        height: 3.5,
                        mass: 1.0e3,
                        stiffness: 5.0e5,
                    }));
            }
            for i in (0..n).rev() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}F", i + 1));
                    ui.add(egui::DragValue::new(&mut self.model.stories[i].height).speed(0.01));
                    ui.add(egui::DragValue::new(&mut self.model.stories[i].mass).speed(10.0));
                    ui.add(egui::DragValue::new(&mut self.model.stories[i].stiffness).speed(10.0));
                    if ui
                        .add_enabled(can_remove, egui::Button::new("remove"))
                        .clicked()
                    {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(index) = to_remove {
                self.model.remove_story(index);
            }
            if ui.button("保存").clicked() {
                *self.save_state.lock().unwrap() = SaveState::Saving;

                let save_state = self.save_state.clone();
                let ctx = ui.ctx().clone();
                let request =
                    ehttp::Request::post_json("http://127.0.0.1:3000/models", &self.model).unwrap();

                ehttp::fetch(request, move |result| {
                    *save_state.lock().unwrap() = match result {
                        Ok(r) if r.status == 201 => SaveState::Success,
                        Ok(r) => SaveState::Error(format!("HTTP {}", r.status)),
                        Err(e) => SaveState::Error(e),
                    };
                    // ctx.request_repaint();
                });
            }
            match &*self.save_state.lock().unwrap() {
                SaveState::Idle => {}
                SaveState::Saving => {
                    ui.label("保存中");
                }
                SaveState::Success => {
                    ui.label("保存成功");
                }
                SaveState::Error(e) => {
                    ui.label(format!("保存失敗 : {}", e));
                }
            };

            ui.allocate_space(ui.available_size());
        });
    }
}
