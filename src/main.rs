#![allow(dead_code)]

mod accessory;
mod chao;
mod chaoface;
mod chaoparam;
mod chaoshape;
mod chaostate;
mod config;
mod drawable;
mod genericjson;
mod ninja;
mod project;

use chaoparam::ChaoParamGc;
use chaostate::ChaoGlobalState;
use eframe::CreationContext;
use egui::mutex::Mutex;
use egui::{Color32, Event, Key};
use egui_wgpu::{WgpuSetup, WgpuSetupCreateNew};
use ninja::math::NinjaRotation;
use ninja::ninjadraw::{NinjaDrawState, NinjaState};
use project::Project;
use self_update::cargo_crate_version;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wgpu::{Device, Features};

use eframe::egui_wgpu::{self, wgpu};

use crate::config::Config;
use crate::project::accessoryedit::AccessoryEditProject;

struct GameState {
    config: Config,
    has_update: Result<bool, self_update::errors::Error>,
    drag_angle: NinjaRotation,
    dist: f32,
    ninja_state: Rc<RefCell<NinjaState>>,
    chao_global_state: Rc<ChaoGlobalState>,
    current_project: Option<Box<dyn Project>>,
    to_be_project: Option<Box<dyn Project>>,
}

impl eframe::App for GameState {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.input(|i| {
            for e in &i.events {
                match e {
                    Event::Key {
                        key,
                        physical_key: _,
                        pressed,
                        repeat,
                        modifiers: _,
                    } => {
                        if *pressed || *repeat {
                            match key {
                                Key::W => self.dist += 40.0 * i.stable_dt,
                                Key::S => self.dist -= 40.0 * i.stable_dt,
                                _ => continue,
                            }

                            if self.dist < 0.0 {
                                self.dist = 0.0;
                            }
                        }
                    }
                    _ => continue,
                }
            }
        });

        self.render(&frame.wgpu_render_state().unwrap().device);

        let mut delete_dialog = false;
        if let Some(proj) = &mut self.to_be_project {
            let open_result = proj.open_dialog(ctx, frame);
            if let Ok(result) = open_result {
                delete_dialog = !result;
            } else if let Err(err) = open_result {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Error")
                    .set_description(format!("Operation failed: {}", err))
                    .show();

                delete_dialog = true;
            }
        }

        if delete_dialog {
            self.to_be_project = None;
        } else if self.to_be_project.is_some() {
            self.current_project = self.to_be_project.take();
        }

        let request_redraw = if let Some(proj) = &mut self.current_project {
            proj.request_redraw()
        } else {
            false
        };

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Accessory JSON").clicked() {
                        self.to_be_project = Some(Box::new(AccessoryEditProject::new(
                            &self.chao_global_state,
                            true,
                        )));
                        ui.close_menu();
                    }

                    if ui.button("Open Accessory JSON").clicked() {
                        self.to_be_project = Some(Box::new(AccessoryEditProject::new(
                            &self.chao_global_state,
                            false,
                        )));
                        ui.close_menu();
                    }
                });

                ui.menu_button("Options", | ui| {
                    let mut changed_config = false;

                    changed_config = ui.checkbox(&mut self.config.auto_save, "Autosave").changed() || changed_config;
                    changed_config = ui.checkbox(&mut self.config.auto_load_found_texture, "Always load automatically detected PAK from gd_PC/PRS").changed() || changed_config;

                    if changed_config {
                        self.config.save_config();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Ok(has_update) = self.has_update {
                        if has_update {
                            ui.hyperlink_to(
                                "Update available!",
                                "https://github.com/Exant64/cweedit/releases/latest",
                            );
                        }
                    } else {
                        ui.colored_label(
                            Color32::RED,
                            format!(
                                "Failed to fetch update! {}",
                                self.has_update.as_ref().err().unwrap()
                            ),
                        );
                    }
                });
            })
        });
        egui::SidePanel::left("side_panel").show_animated(ctx, true, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(proj) = &mut self.current_project {
                    proj.side_panel(ctx, frame, ui);
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    self.custom_painting(ui);
                });
            });
        });

        if request_redraw {
            ctx.request_repaint();
        }
    }
}

struct NinjaViewportPainter {}

impl egui_wgpu::CallbackTrait for NinjaViewportPainter {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let draw_state: &mut Arc<Mutex<NinjaDrawState>> = _callback_resources.get_mut().unwrap();
        draw_state.lock().set_buffers(_device, _queue);
        Vec::new()
    }

    fn finish_prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let draw_state: &Arc<Mutex<NinjaDrawState>> = callback_resources.get().unwrap();
        draw_state.lock().draw_entries(render_pass);
    }
}

impl GameState {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        let drag_speed = 100.0;
        self.drag_angle.y += (drag_speed * _response.drag_motion().x) as i32;
        self.drag_angle.x += (drag_speed * _response.drag_motion().y) as i32;

        self.ninja_state
            .borrow_mut()
            .draw_state
            .lock()
            .set_projection_matrix(rect.aspect_ratio());

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            NinjaViewportPainter {},
        ));
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ninja_state = NinjaState::init(cc).unwrap();

        let chao_global_state = Rc::new(ChaoGlobalState::init(
            cc.wgpu_render_state.as_ref().unwrap(),
        ));

        cc.wgpu_render_state
            .as_ref()
            .unwrap()
            .renderer
            .write()
            .callback_resources
            .insert(ninja_state.draw_state.clone());

        let has_update = self_update::backends::github::Update::configure()
            .repo_owner("exant64")
            .repo_name("cweedit")
            .bin_name("cweedit")
            .show_download_progress(true)
            .current_version(cargo_crate_version!())
            .build()
            .and_then(|config| {
                config.get_latest_release().and_then(|release| {
                    self_update::version::bump_is_greater(cargo_crate_version!(), &release.version)
                })
            });

        let config = Config::init().unwrap_or_else(|error_msg| {
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Error")
                .set_description(format!("Failed to read editor config: {}", error_msg))
                .show();

            Config::default()
        });

        Self {
            config,
            has_update,
            current_project: None,
            to_be_project: None,
            drag_angle: NinjaRotation { x: 0, y: 0, z: 0 },
            dist: 0.0,
            ninja_state: Rc::new(RefCell::new(ninja_state)),
            chao_global_state: chao_global_state.clone(),
        }
    }

    // sets up viewmatrix and before_draw
    fn setup_frame(&mut self) {
        let mut ninja = self.ninja_state.borrow_mut();

        ninja.draw_state.lock().clear_buffers();

        ninja.matrix_stack.push_matrix(&glam::Mat4::look_at_rh(
            glam::Vec3::new(5.0, 0.0, self.dist - 10.0),
            glam::Vec3::ZERO,
            glam::Vec3::Y,
        ));
    }

    fn finish_frame(&mut self) {
        let mut ninja = self.ninja_state.borrow_mut();

        ninja.matrix_stack.pop();
    }

    pub fn render(&mut self, device: &Device) {
        self.setup_frame();
        self.ninja_state.borrow_mut().matrix_stack.push();

        self.ninja_state
            .borrow_mut()
            .matrix_stack
            .rotate(&self.drag_angle);

        if let Some(drawable) = self
            .current_project
            .as_mut()
            .and_then(|proj| proj.get_drawable())
        {
            drawable.draw(device, &self.ninja_state);
        }

        self.ninja_state.borrow_mut().matrix_stack.pop();
        self.finish_frame();
    }
}

pub fn main() {
    let mut options = eframe::NativeOptions {
        depth_buffer: 32,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 1024.0])
            .with_drag_and_drop(true),

        renderer: eframe::Renderer::Wgpu,

        ..Default::default()
    };

    let mut setup: WgpuSetupCreateNew = WgpuSetupCreateNew {
        ..Default::default()
    };

    setup.device_descriptor = Arc::new(|_| {
        let mut descriptor = wgpu::DeviceDescriptor {
            ..Default::default()
        };
        descriptor.required_features |= Features::TEXTURE_COMPRESSION_BC;
        descriptor
    });

    if let WgpuSetup::CreateNew(orig_setup) = &options.wgpu_options.wgpu_setup {
        setup.instance_descriptor = orig_setup.instance_descriptor.clone();
    }

    options.wgpu_options.wgpu_setup = WgpuSetup::CreateNew(setup);

    eframe::run_native(
        &format!("Chao World Extended Editor ({})", cargo_crate_version!()),
        options,
        Box::new(|cc: &CreationContext<'_>| {
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            Ok(Box::new(GameState::new(cc)))
        }),
    )
    .expect("Couldn't start egui app!");
}
