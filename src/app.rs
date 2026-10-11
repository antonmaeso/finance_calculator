//! Top-level application state, layout and persistence.

use std::collections::BTreeMap;

use eframe::egui::{self, RichText};
use serde::{Deserialize, Serialize};

use crate::ui::inputs::inputs_panel;
use crate::ui::tabs::{self, Computed, Tab, TabState};
use crate::ui::{help, MUTED};
use house_calc::model::Inputs;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct App {
    inputs: Inputs,
    tabs: TabState,
    saved: BTreeMap<String, Inputs>,
    save_name: String,
    file_path: String,
    #[serde(skip)]
    computed: Computed,
    #[serde(skip)]
    status: String,
}

impl Default for App {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            inputs: Inputs::default(),
            tabs: TabState::default(),
            saved: BTreeMap::new(),
            save_name: "My plan".into(),
            file_path: format!("{home}/house_move_scenario.json"),
            computed: Computed::default(),
            status: String::new(),
        }
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut app: Self = cc.storage.and_then(|s| eframe::get_value(s, eframe::APP_KEY)).unwrap_or_default();
        // Open a specific tab on launch, e.g. HOUSE_CALC_TAB=3 (handy for screenshots).
        if let Some(t) = std::env::var("HOUSE_CALC_TAB").ok().and_then(|v| v.parse::<usize>().ok()) {
            app.tabs.tab = Tab::ALL[t.min(Tab::ALL.len() - 1)];
        }
        // Load inputs from a JSON file on launch, e.g. HOUSE_CALC_INPUTS=my_plan.json.
        if let Some(i) = std::env::var("HOUSE_CALC_INPUTS")
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Inputs>(&s).ok())
        {
            app.inputs = i;
        }
        app
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Scottish house move calculator");
            ui.separator();
            let mut h = self.inputs.horizon_years;
            if ui.add(egui::Slider::new(&mut h, 1..=30).text("years to compare")).changed() {
                self.inputs.horizon_years = h;
            }
            ui.checkbox(&mut self.inputs.liquidate_at_end, "Count selling costs & CGT on first home at the end")
                .on_hover_text("Compare what you'd actually walk away with if the first home were sold at the end.");
            ui.separator();
            ui.label(RichText::new(format!("Tax rules {}", self.inputs.tax.year_label)).color(MUTED));
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Scenario:");
            ui.add(egui::TextEdit::singleline(&mut self.save_name).desired_width(120.0));
            if ui.button("💾 Save").clicked() && !self.save_name.trim().is_empty() {
                self.saved.insert(self.save_name.trim().to_string(), self.inputs.clone());
                self.status = format!("Saved “{}”", self.save_name.trim());
            }
            let mut load = None;
            let mut delete = None;
            egui::ComboBox::from_id_salt("load").selected_text("📂 Load…").show_ui(ui, |ui| {
                for name in self.saved.keys() {
                    ui.horizontal(|ui| {
                        if ui.button(name).clicked() {
                            load = Some(name.clone());
                        }
                        if ui.small_button("🗑").clicked() {
                            delete = Some(name.clone());
                        }
                    });
                }
                if self.saved.is_empty() {
                    ui.label("No saved scenarios yet");
                }
            });
            if let Some(n) = load {
                self.inputs = self.saved[&n].clone();
                self.save_name = n.clone();
                self.status = format!("Loaded “{n}”");
            }
            if let Some(n) = delete {
                self.saved.remove(&n);
                self.status = format!("Deleted “{n}”");
            }
            if ui.button("↺ Defaults").clicked() {
                self.inputs = Inputs::default();
                self.status = "Reset to example values".into();
            }
            ui.separator();
            ui.add(egui::TextEdit::singleline(&mut self.file_path).desired_width(240.0));
            if ui.button("Export JSON").clicked() {
                self.status = match serde_json::to_string_pretty(&self.inputs).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&self.file_path, s).map_err(|e| e.to_string())) {
                    Ok(()) => format!("Exported to {}", self.file_path),
                    Err(e) => format!("Export failed: {e}"),
                };
            }
            if ui.button("Import JSON").clicked() {
                self.status = match std::fs::read_to_string(&self.file_path).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Inputs>(&s).map_err(|e| e.to_string())) {
                    Ok(i) => {
                        self.inputs = i;
                        format!("Imported {}", self.file_path)
                    }
                    Err(e) => format!("Import failed: {e}"),
                };
            }
            if !self.status.is_empty() {
                ui.label(RichText::new(&self.status).color(MUTED));
            }
        });
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(4.0);
        });

        egui::Panel::left("inputs").resizable(true).default_size(380.0).min_size(300.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);
                help(ui, "Drag sliders or click a number to type it. Hover for explanations.");
                inputs_panel(ui, &mut self.inputs);
                ui.add_space(20.0);
            });
        });

        self.computed.refresh(&self.inputs);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for t in Tab::ALL {
                    ui.selectable_value(&mut self.tabs.tab, t, RichText::new(t.label()).size(15.0));
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let (i, c) = (&self.inputs, &self.computed);
                match self.tabs.tab {
                    Tab::Overview => tabs::overview(ui, i, c),
                    Tab::Mortgages => tabs::mortgages(ui, i, c, &mut self.tabs),
                    Tab::Letting => tabs::letting(ui, i, c),
                    Tab::Taxes => tabs::taxes(ui, i, c),
                    Tab::SellAndSave => tabs::sell_and_save(ui, i, c),
                    Tab::Sensitivity => tabs::sensitivity(ui, i, &mut self.tabs),
                    Tab::Considerations => tabs::considerations(ui, i, c),
                }
            });
        });
    }
}
