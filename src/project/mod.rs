pub mod accessoryedit;
pub mod drawable;

use std::path::PathBuf;

use drawable::Drawable;
use rfd::FileDialog;

pub fn show_error(text: impl Into<String>) {
    rfd::MessageDialog::new()
        .set_title("Error")
        .set_description(text)
        .set_level(rfd::MessageLevel::Error)
        .show();
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
    fn open_dialog(&mut self, ctx: &egui::Context, frame: &eframe::Frame) -> Result<bool, String>;
    fn side_panel(&mut self, ctx: &egui::Context, frame: &eframe::Frame, ui: &mut egui::Ui);
    fn request_redraw(&self) -> bool;
    fn get_drawable(&mut self) -> Option<&mut dyn Drawable>;
}
