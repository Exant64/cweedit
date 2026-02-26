pub mod accessoryedit;
pub mod drawable;

use std::path::PathBuf;

use drawable::Drawable;
use egui::{Color32, RichText, Ui};
use rfd::FileDialog;

use crate::config::Config;

pub fn show_error(text: impl Into<String>) {
    rfd::MessageDialog::new()
        .set_title("Error")
        .set_description(text)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

// the inspiration from this is from encounter's "objdiff" project (similarly made in egui, but not directly using his code)
// thanks!
pub fn tooltip_helper(
    ui: &mut Ui,
    ui_element: impl FnOnce(&mut Ui),
    tooltip: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui_element(ui);

        let resp = ui.label(RichText::new("\u{2139}").color(Color32::CYAN));
        if resp.hovered() {
            resp.show_tooltip_ui(tooltip);
        }
    });
}

pub fn open_file_dialog(file_name: &'static str, extensions: &[&'static str]) -> Option<PathBuf> {
    FileDialog::new()
        .set_title(file_name)
        .add_filter(file_name, extensions)
        .pick_file()
}

pub fn save_file_dialog(file_name: &'static str, extensions: &[&'static str]) -> Option<PathBuf> {
    FileDialog::new()
        .set_title(file_name)
        .add_filter(file_name, extensions)
        .save_file()
}

pub trait Project {
    fn unsaved_changes(&self) -> bool;
    fn open_dialog(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        config: &Config,
    ) -> Result<bool, String>;
    fn side_panel(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        ui: &mut egui::Ui,
        config: &Config,
    );
    fn request_redraw(&self) -> bool;
    fn get_drawable(&mut self) -> Option<&mut dyn Drawable>;
}
