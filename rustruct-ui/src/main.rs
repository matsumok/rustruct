use eframe::egui;

fn main() -> eframe::Result {
    eframe::run_native(
        "Rustruct",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(RustructApp::default()))),
    )
}

#[derive(Default)]
struct RustructApp {
    model_name: String,
    count: usize,
}

impl eframe::App for RustructApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Rustruct");
            ui.text_edit_singleline(&mut self.model_name);
            ui.text_edit_singleline(&mut self.model_name);
            if ui.button("+").clicked() {
                self.count += 1;
            }
            ui.label(format!("model: {} count: {}", self.model_name, self.count));
        });
    }
}
