// The #[run_example] macro generates:
//   - wasm32: a #[wasm_bindgen(start)] that calls this function body
//   - native: a main with `dist` / `start` sub-commands that build the wasm
//             bundle and serve it via a local dev server
#[xtask_wasm::run_example(assets_dir = "assets")]
fn run() {
    use eframe::egui;
    use egui_nonogram::{GameStatus, NonogramGame, NonogramWidget};
    use serde::{Deserialize, Serialize};
    use xtask_wasm::wasm_bindgen::JsCast as _;

    const SELECTED_PRESET_KEY: &str = "selected_preset";

    #[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
    enum Preset {
        Beginner,
        Intermediate,
        Expert,
    }

    impl Preset {
        const ALL: &'static [Preset] = &[Self::Beginner, Self::Intermediate, Self::Expert];

        fn label(self) -> &'static str {
            match self {
                Self::Beginner => "Beginner (8x8)",
                Self::Intermediate => "Intermediate (12x12)",
                Self::Expert => "Expert (16x16)",
            }
        }

        fn dims(self) -> (usize, usize, f32) {
            match self {
                Self::Beginner => (8, 8, 0.5),
                Self::Intermediate => (12, 12, 0.45),
                Self::Expert => (16, 16, 0.4),
            }
        }
    }

    struct NonogramApp {
        game: NonogramGame,
        selected_preset: Preset,
        seed_counter: u64,
    }

    impl eframe::App for NonogramApp {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let bg = ui.max_rect();
            ui.painter()
                .rect_filled(bg, egui::CornerRadius::ZERO, ui.visuals().panel_fill);

            egui::Panel::top("top_bar")
                .frame(egui::Frame::new().inner_margin(4.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.visuals_mut().button_frame = false;
                        ui.add_space(8.0);
                        egui::widgets::global_theme_preference_switch(ui);
                        ui.separator();
                        for &preset in Preset::ALL {
                            if ui
                                .selectable_label(self.selected_preset == preset, preset.label())
                                .clicked()
                            {
                                self.new_game(preset);
                            }
                        }
                        ui.separator();
                        if ui
                            .add_enabled(self.game.can_undo(), egui::Button::new("Undo"))
                            .clicked()
                        {
                            self.game.undo();
                        }
                        if ui
                            .add_enabled(self.game.can_redo(), egui::Button::new("Redo"))
                            .clicked()
                        {
                            self.game.redo();
                        }
                        if self.game.status == GameStatus::Won {
                            ui.colored_label(egui::Color32::GREEN, "Solved!");
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("New Game").clicked() {
                                let preset = self.selected_preset;
                                self.new_game(preset);
                            }
                        });
                    });
                });

            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add(NonogramWidget::new(&mut self.game));
            });
        }

        fn save(&mut self, storage: &mut dyn eframe::Storage) {
            eframe::set_value(storage, SELECTED_PRESET_KEY, &self.selected_preset);
        }
    }

    impl NonogramApp {
        fn new_game(&mut self, preset: Preset) {
            self.selected_preset = preset;
            let (w, h, density) = preset.dims();
            self.seed_counter += 1;
            self.game = NonogramGame::random(w, h, density, self.seed_counter);
        }

        fn new(cc: &eframe::CreationContext<'_>) -> Self {
            let selected_preset = cc
                .storage
                .and_then(|storage| eframe::get_value(storage, SELECTED_PRESET_KEY))
                .unwrap_or(Preset::Beginner);
            let (w, h, density) = selected_preset.dims();

            Self {
                game: NonogramGame::random(w, h, density, 1),
                selected_preset,
                seed_counter: 1,
            }
        }
    }

    // Create a full-screen canvas and attach it to the page body.
    let document = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document");

    let canvas = document
        .create_element("canvas")
        .expect("failed to create canvas")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("not a HtmlCanvasElement");

    let style = canvas.style();
    style.set_property("position", "fixed").unwrap();
    style.set_property("top", "0").unwrap();
    style.set_property("left", "0").unwrap();
    style.set_property("width", "100%").unwrap();
    style.set_property("height", "100%").unwrap();

    let body = document.body().expect("no body");
    body.style().set_property("margin", "0").unwrap();
    body.append_child(&canvas).expect("failed to append canvas");
    canvas.style().set_property("touch-action", "none").unwrap();

    // Start the eframe web runner on that canvas element.
    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(NonogramApp::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
