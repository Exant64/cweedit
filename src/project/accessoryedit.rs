use std::{collections::HashMap, path::PathBuf, rc::Rc, str::FromStr};

use egui::Color32;
use egui_wgpu::RenderState;
use rfd::MessageDialogResult;

use crate::{
    accessory::{AccessoryData, AccessoryType, BaldMode},
    chao::{
        BALD_DEFAULT_CENTER, BALD_DEFAULT_CLIP_FACE, BALD_DEFAULT_INFLUENCE, BALD_DEFAULT_RADIUS,
    },
    chaostate::ChaoGlobalState,
    config::Config,
    genericjson::MarketData,
    ninja::{
        math::Color, modelfile::NinjaChunkObject, polychunk::PolyChunk, texlist::NinjaGpuTexEntry,
        texture::gvm::NinjaTexlist,
    },
    project::{save_file_dialog, show_error},
};

use super::{
    drawable::{chaodraw::ChaoDraw, Drawable},
    open_file_dialog, tooltip_helper, Project,
};

const CHANGED_VISUAL: u32 = 1 << 0;
const CHANGED_FILE: u32 = 1 << 1;
const CHANGED_ALL: u32 = CHANGED_VISUAL | CHANGED_FILE;

pub struct AccessoryEditProject {
    disable_accessory_preview: bool,
    hide_parts_selected_num: u8,
    open_or_save: bool,
    id: String,
    json_path: Option<PathBuf>,
    chao_draw: ChaoDraw,
    object_path: Option<PathBuf>,
    texture_name: Option<String>,
    object: Option<NinjaChunkObject>,
    texlist: Option<Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>>,
    accessory_data: Option<AccessoryData>,
    hide_parts: Vec<u8>,
    disable_jiggle: bool,
    accessory_type: AccessoryType,
    market_data: MarketData,
    bald_mode: BaldMode,
    bald_dont_hide_hparts: bool,
    bald_preset_sides: [bool; 3],
    bald_influence: glam::Vec3,
    bald_center: glam::Vec3,
    bald_radius: f32,
    bald_clip_face: bool,

    use_renderfix: bool,
    renderfix_preview: bool,

    material_highlight_node_select: Option<usize>,
    material_highlight_material_select: Option<usize>,
    material_backup_color: HashMap<(usize, usize), Color>,
    material_flash: bool,

    selected_slot: usize,
    material_slot_users: HashMap<(usize, usize), usize>,
    material_slots: [Color; 8],

    unsaved_changes: bool,
}

impl Project for AccessoryEditProject {
    fn request_redraw(&self) -> bool {
        self.material_flash || self.chao_draw.refresh_every_frame()
    }

    fn get_drawable(&mut self) -> Option<&mut dyn Drawable> {
        Some(&mut self.chao_draw)
    }

    fn open_dialog(&mut self, ctx: &egui::Context, frame: &eframe::Frame) -> Result<bool, String> {
        if self.json_path.is_some() {
            return Ok(true);
        }

        if !self.open_or_save {
            let dialog_result = open_file_dialog("Accessory JSON file", &["json"]);
            if dialog_result.is_none() {
                return Ok(false);
            }
            let path = dialog_result.unwrap();

            let (accessory_data, market_data) = AccessoryData::read_json(ctx, frame, &path)?;

            self.id = accessory_data.id;
            self.object_path = accessory_data.object_path;
            self.texture_name = accessory_data.texture_name;
            self.object = Some(accessory_data.object);
            self.texlist = Some(accessory_data.texlist);
            self.disable_jiggle = accessory_data.disable_jiggle;
            self.accessory_type = accessory_data.accessory_type;
            self.hide_parts = accessory_data.hide_parts;
            self.bald_preset_sides = accessory_data.bald_preset_sides;
            self.bald_dont_hide_hparts = accessory_data.bald_dont_hide_parts;
            self.bald_mode = accessory_data.bald_mode;
            self.bald_center = accessory_data.bald_center;
            self.bald_influence = accessory_data.bald_influence;
            self.bald_clip_face = accessory_data.bald_clip_face;
            self.bald_radius = accessory_data.bald_radius;
            self.material_slot_users = accessory_data.material_slot_users;
            self.material_slots = accessory_data.material_slots;

            self.market_data = market_data;

            self.json_path = Some(path);
            self.check_update();

            Ok(true)
        } else {
            let dialog_result = save_file_dialog("Accessory JSON file", &["json"]);
            if let Some(path) = dialog_result {
                if let Some(extension) = path.extension() {
                    if let Some(extension_str) = extension.to_str() {
                        if extension_str != "json" {
                            return Err("Selected file is not a .json!".to_string());
                        }
                    } else {
                        return Err(format!(
                            "Failed to convert extension OsStr to str! ({:?})",
                            extension
                        ));
                    }
                } else {
                    return Err("Selected file has no extension (needs to be .json)!".to_string());
                }

                if let Some(parent) = path.parent() {
                    if !parent.ends_with("CWE/Accessories") {
                        return Err("JSON file is not in CWE/Accessories folder!".to_string());
                    }
                } else {
                    return Err("Failed to retrieve parent folders of chosen file! File needs to be saved in a CWE/Accessories directory.".to_string());
                }

                if std::fs::exists(&path).map_err(|_| "Failed to check file! Permission denied?")? {
                    return Err("File already exists! Choose another filename, or remove said file before creating it.".into());
                }

                self.json_path = Some(path);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    fn side_panel(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        ui: &mut egui::Ui,
        config: &Config,
    ) {
        let changed = self.panel_elements(ctx, frame, ui, config);

        if (changed & CHANGED_VISUAL) != 0 {
            self.check_update();
        }

        if (changed & CHANGED_FILE) != 0 {
            if !config.auto_save {
                self.unsaved_changes = true;
            } else {
                if let Some(data) = &self.accessory_data {
                    if let Some(json_path) = &self.json_path {
                        if !self.id.is_empty() {
                            let json_err = data.save_json(&self.market_data, json_path);
                            if json_err.is_err() {
                                show_error(format!(
                                    "Failed to save json: {}",
                                    json_err.err().unwrap()
                                ));
                            } else {
                                self.unsaved_changes = false;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl AccessoryEditProject {
    fn panel_elements(
        &mut self,
        ctx: &egui::Context,
        frame: &eframe::Frame,
        ui: &mut egui::Ui,
        config: &Config,
    ) -> u32 {
        let mut changed: u32 = 0;

        ui.heading("Preview Settings");
        ui.collapsing("Chao Preview", |ui| {
            self.chao_draw.chao_preview_edit(ui, ctx);
        });

        tooltip_helper(
            ui,
            |ui| {
                if ui
                    .checkbox(
                        &mut self.disable_accessory_preview,
                        "Disable Accessory Preview",
                    )
                    .changed()
                {
                    changed |= CHANGED_VISUAL;
                }
            },
            |ui| {
                ui.label("Disables showing the accessory on the Chao in the preview panel");
            },
        );

        if ui
            .checkbox(&mut self.renderfix_preview, "Render Fix Preview")
            .changed()
        {
            changed |= CHANGED_VISUAL;
        }

        ui.separator();

        ui.heading("Accessory Settings");

        tooltip_helper(
            ui,
            |ui| {
                if ui
                    .checkbox(&mut self.use_renderfix, "Supports Render Fix")
                    .changed()
                {
                    changed |= CHANGED_ALL;
                }
            },
            |ui| {
                ui.label("Enables Render Fix \"Normal Draw\" support for the accessory. This is recommended!\nIt enables proper Ambient and Specular material color support (including the exponent for specular values), texture filter options, vertex colored accessories, and proper double-sided lighting.");
            },
        );

        tooltip_helper(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("ID: ").color(if self.id.is_empty() {
                        egui::Color32::from_rgb(255, 50, 50)
                    } else {
                        egui::Color32::from_rgb(50, 255, 50)
                    }));

                    if ui.add(egui::TextEdit::singleline(&mut self.id)).changed() {
                        self.id.truncate(20);
                        if let Some(data) = &mut self.accessory_data {
                            data.id = self.id.clone();
                            changed |= CHANGED_ALL;
                        }
                    }
                });
            },
            |ui| {
                ui.label("This is a unique identifier for each accessory (max length: 20).\nMake sure this is as unique to your mod as possible, if your accessory is just a hat, don't use the ID \"hat\", name it something more like \"jimsaccmodhat\".");
            },
        );

        if let Some(obj) = &self.object {
            tooltip_helper(
                ui,
                |ui| {
                    if ui.button("Generate ID from Object Hash").clicked() {
                        let diag_result = rfd::MessageDialog::new()
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .set_title("Generate ID from Object Hash")
                        .set_description("We only recommend using this if you plan to update/replace accessories in an existing DLL-based accessory mod of yours. If this is a brand new accessory, write a unique ID instead! Are you sure you want to proceed?")
                        .show();

                        if diag_result == MessageDialogResult::Yes {
                            if let Some(hash) = obj.get_hash() {
                                let new_id = format!("acc{:x}", hash);

                                self.id = new_id;
                                if let Some(data) = &mut self.accessory_data {
                                    data.id = self.id.clone();
                                    changed |= CHANGED_ALL;
                                }
                            }
                        }
                    }
                },
                |ui| {
                    ui.label("This is only intended for old DLL-based accessory mods being remade in the editor. If this is a brand new accessory, write a unique ID instead!");
                },
            );
        }

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Object: ").color(if self.object_path.is_none() {
                    egui::Color32::from_rgb(255, 50, 50)
                } else {
                    egui::Color32::from_rgb(50, 255, 50)
                }),
            );

            if ui.button("Select").clicked() {
                if let Some(json_path) = &self.json_path {
                    if let Some(picked) = open_file_dialog("SA2MDL file", &["sa2mdl"]) {
                        if let Err(str) = AccessoryData::safety_check_object_path_before_save(
                            picked.as_path(),
                            json_path.as_path(),
                        ) {
                            show_error(str);
                        } else {
                            let picked_clone = picked.clone();
                            if self.object_path != Some(picked_clone) {
                                self.object = NinjaChunkObject::read_file(&picked.clone()).ok();
                                self.object_path = Some(picked);

                                self.material_backup_color.clear();
                                self.material_highlight_material_select = None;
                                self.material_highlight_node_select = None;
                                self.material_slot_users.clear();

                                changed |= CHANGED_ALL;
                            }
                        }
                    }
                }
            }

            if let Some(path) = &self.object_path {
                if ui.button("Reload").clicked() {
                    self.object = NinjaChunkObject::read_file(path).ok();
                    changed |= CHANGED_VISUAL;
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Texture: ").color(if self.texture_name.is_none() {
                    egui::Color32::from_rgb(255, 50, 50)
                } else {
                    egui::Color32::from_rgb(50, 255, 50)
                }),
            );

            if ui.button("Select").clicked() {
                // disgusting nest
                if let Some(path) = open_file_dialog("Texture file", &["pak", "gvm"]) {
                    if let Some(tex_state) = frame.wgpu_render_state() {
                        if let Some(path_file_name) = path.file_stem() {
                            if let Some(path_str) = path_file_name.to_str() {
                                if let Ok(texlist) = NinjaTexlist::load_tex(tex_state, &path) {
                                    self.texlist = Some(Rc::new(texlist));
                                    self.texture_name = Some(path_str.to_string());

                                    changed |= CHANGED_ALL;
                                } else {
                                    show_error("Failed to load texture! Invalid format?");
                                }
                            } else {
                                show_error(
                                    "Failed to convert path filename to string! (invalid unicode?)",
                                );
                            }
                        } else {
                            show_error("Failed to retrieve filename from path!");
                        }
                    } else {
                        show_error("Failed to retrieve wgpu_render_state for loading texture!");
                    }
                }
            }
        });

        let prev_accessory_type = self.accessory_type.clone();
        tooltip_helper(
            ui,
            |ui| {
                egui::ComboBox::from_label("Accessory Type")
                    .selected_text(format!("{:?}", self.accessory_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.accessory_type, AccessoryType::Head, "Head");
                        ui.selectable_value(&mut self.accessory_type, AccessoryType::Face, "Face");
                        if let Some(accessory) = &mut self.accessory_data {
                            if accessory.check_if_generic_appropriate() {
                                ui.selectable_value(
                                    &mut self.accessory_type,
                                    AccessoryType::Generic1,
                                    "Generic1",
                                );
                                ui.selectable_value(
                                    &mut self.accessory_type,
                                    AccessoryType::Generic2,
                                    "Generic2",
                                );
                            }
                        }
                    });
            },
            |ui| {
                ui.label("This specifies the slot the Chao will put the accessory in.\nFor Head and Face type, the accessory model gets parented to the chao's head (node 16).\nFor Generic types, the Chao model behaves like a rig the accessory binds to.");
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("Visit the ");
                    ui.hyperlink_to("Chao World modding documentation", "https://nostalgianinja.github.io/ChaoModding_Docs/AccessoryModding/#body-accessories");
                    ui.label(" to learn more");
                });
            },
        );

        if prev_accessory_type != self.accessory_type {
            changed |= CHANGED_ALL;
        }

        let bald_mode_prev = self.bald_mode.clone();
        tooltip_helper(
            ui,
            |ui| {
                egui::ComboBox::from_label("Bald Mode")
                    .selected_text(format!("{:?}", self.bald_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.bald_mode, BaldMode::None, "None");
                        ui.selectable_value(&mut self.bald_mode, BaldMode::Presets, "Presets");
                        ui.selectable_value(&mut self.bald_mode, BaldMode::Custom, "Custom");
                    });
            },
            |ui| {
                ui.label("This option flattens the Chao's head to prevent clipping issues with evolution head shapes. It also hides the other Chao head parts (ex. the \"bulbs\" on Hero Chao) by default (this is configurable).\n\
                The \"Presets\" mode has a preset configuration that you can toggle for each axis (basically letting you shrink the chao's head on each axis).\n\
                The \"Custom\" mode allows you to configure the parameters of the \"shrinking\" more precisely. Check the help tooltips on the parameters for more info on these.");
            },
        );

        if bald_mode_prev != self.bald_mode {
            changed |= CHANGED_ALL;
        }

        match self.bald_mode {
            BaldMode::None => {}
            BaldMode::Presets => {
                ui.collapsing("Bald Settings", |ui| {
                    let mut bald_changed = false;
                    bald_changed = ui
                        .checkbox(&mut self.bald_dont_hide_hparts, "Don't hide head parts")
                        .changed()
                        || bald_changed;
                    bald_changed = ui
                        .checkbox(&mut self.bald_preset_sides[0], "Side (X)")
                        .changed()
                        || bald_changed;
                    bald_changed = ui
                        .checkbox(&mut self.bald_preset_sides[1], "Top (Y)")
                        .changed()
                        || bald_changed;
                    bald_changed = ui
                        .checkbox(&mut self.bald_preset_sides[2], "Back (Z)")
                        .changed()
                        || bald_changed;

                    if bald_changed {
                        changed |= CHANGED_ALL;
                    }
                });
            }
            BaldMode::Custom => {
                ui.collapsing("Advanced Bald Settings", |ui| {
                    if ui.checkbox(
                        &mut self.bald_dont_hide_hparts,
                        "Don't hide head parts",
                    ).changed() {
                        changed |= CHANGED_ALL;
                    }

                    tooltip_helper(
                        ui,
                        |ui| {
                            ui.label("Information");
                        },
                        |ui| {
                            ui.label(
                                "The \"bald\" system works by morphing the Chao head vertices to the nearest point on a sphere.\n\
                                You can configure the radius and the center of this sphere, and also the \"strength\" of the morphing on each axis.",
                            );
                        },
                    );

                    tooltip_helper(
                        ui,
                        |ui| {
                            ui.label("Center");
                        },
                        |ui| {
                            ui.label(
                                "The center of the sphere that the Chao head gets fitted into.",
                            );
                        },
                    );

                    ui.horizontal(|ui| {
                        let mut slider_changed = false;
                        slider_changed = ui.add(egui::Slider::new(&mut self.bald_center.x, -2.0..=2.0)).changed() || slider_changed;
                        slider_changed = ui.add(egui::Slider::new(&mut self.bald_center.y, -2.0..=2.0)).changed() || slider_changed;
                        slider_changed = ui.add(egui::Slider::new(&mut self.bald_center.z, -2.0..=2.0)).changed() || slider_changed;

                        if slider_changed {
                            changed |= CHANGED_ALL;
                        }
                    });

                    tooltip_helper(
                        ui,
                        |ui| {
                            ui.label("Influence");
                        },
                        |ui| {
                            ui.label("The strength of the \"fitting\" on each axis.");
                        },
                    );

                    ui.horizontal(|ui| {
                        let mut slider_changed = false;
                        slider_changed = ui.add(egui::Slider::new(&mut self.bald_influence.x, 0.0..=1.0)).changed() || slider_changed;
                        slider_changed = ui.add(egui::Slider::new(&mut self.bald_influence.y, 0.0..=1.0)).changed() || slider_changed;
                        slider_changed = ui.add(egui::Slider::new(&mut self.bald_influence.z, 0.0..=1.0)).changed() || slider_changed;

                        if slider_changed {
                            changed |= CHANGED_ALL;
                        }
                    });

                    tooltip_helper(
                        ui,
                        |ui| {
                            ui.label("Radius");
                        },
                        |ui| {
                            ui.label(
                                "The radius of the sphere that the Chao head gets fitted into.",
                            );
                        },
                    );

                    if ui.add(egui::Slider::new(&mut self.bald_radius, 0.0..=10.0)).changed() {
                        changed |= CHANGED_ALL;
                    }

                    tooltip_helper(
                        ui,
                        |ui| {
                            if ui.checkbox(&mut self.bald_clip_face, "Clip Face").changed() {
                                changed |= CHANGED_ALL;
                            }
                        },
                        |ui| {
                            ui.label("Enabling this leaves the face unaffected by the fitting.");
                        },
                    );

                    tooltip_helper(
                        ui,
                        |ui| {
                            if ui.button("Reset to Default").clicked() {
                                self.bald_center = BALD_DEFAULT_CENTER;
                                self.bald_clip_face = BALD_DEFAULT_CLIP_FACE;
                                self.bald_influence = glam::vec3(
                                    BALD_DEFAULT_INFLUENCE,
                                    BALD_DEFAULT_INFLUENCE,
                                    BALD_DEFAULT_INFLUENCE,
                                );
                                self.bald_radius = BALD_DEFAULT_RADIUS;
                                self.bald_dont_hide_hparts = false;

                                changed |= CHANGED_ALL;
                            }
                        },
                        |ui| {
                            ui.label("Resets the parameters to default. (these are the same values that the \"Presets\" mode uses)");
                        },
                    );
                });
            }
        };

        if ui
            .checkbox(&mut self.disable_jiggle, "Disable Jiggle")
            .changed()
        {
            changed |= CHANGED_ALL;
        }

        ui.collapsing("Market Data", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                if ui
                    .add(egui::TextEdit::singleline(&mut self.market_data.name))
                    .changed()
                {
                    changed |= CHANGED_ALL;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Description: ");
                if ui
                    .add(egui::TextEdit::singleline(
                        &mut self.market_data.description,
                    ))
                    .changed()
                {
                    changed |= CHANGED_ALL;
                }
            });

            let mut num_str = self.market_data.price.to_string();
            ui.horizontal(|ui| {
                ui.label("Purchase Price: ");
                if ui.text_edit_singleline(&mut num_str).changed() {
                    // if the input value is vaild, update the value
                    if let Ok(parsed_value) = num_str.parse() {
                        self.market_data.price = parsed_value;
                        changed |= CHANGED_ALL;
                    }
                };
            });

            num_str = self.market_data.sale.to_string();
            ui.horizontal(|ui| {
                ui.label("Sale Price: ");
                if ui.text_edit_singleline(&mut num_str).changed() {
                    // if the input value is vaild, update the value
                    if let Ok(parsed_value) = num_str.parse() {
                        self.market_data.sale = parsed_value;
                        changed |= CHANGED_ALL;
                    }
                };
            });

            num_str = self.market_data.emblems.to_string();
            ui.horizontal(|ui| {
                ui.label("Required Emblems: ");
                if ui.text_edit_singleline(&mut num_str).changed() {
                    // if the input value is vaild, update the value
                    if let Ok(parsed_value) = num_str.parse() {
                        self.market_data.emblems = parsed_value;
                        changed |= CHANGED_ALL;
                    }
                };
            });
        });

        ui.collapsing("Hide Parts", |ui| {
            tooltip_helper(
                ui,
                |ui| {
                    ui.label("Information");
                },
                |ui| {
                    ui.label("This sub-menu lets you hide specific Chao nodes (indices 0 through 39). The list also shows the ones automatically hidden by Bald settings (if enabled).");
                },
            );

            let mut num_str = self.hide_parts_selected_num.to_string();
            ui.horizontal(|ui| {
                ui.label("Node: ");
                if ui.text_edit_singleline(&mut num_str).changed() {
                    // if the input value is vaild, update the value
                    if let Ok(parsed_value) = num_str.parse::<u8>() {
                        self.hide_parts_selected_num = parsed_value.min(39);
                    }
                };

                if ui.button("Add").clicked()
                    && !self.hide_parts.contains(&self.hide_parts_selected_num) {
                        self.hide_parts.push(self.hide_parts_selected_num);
                        changed |= CHANGED_ALL;
                    }
            });

            ui.label("Hidden:");

            if self.bald_mode != BaldMode::None && !self.bald_dont_hide_hparts {
                ui.label("23 (hidden by bald automatically)");
                ui.label("25 (hidden by bald automatically)");
            }

            for x in self.hide_parts.clone() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}", x));
                    if (ui.button("Remove")).clicked() {
                        self.hide_parts
                            .remove(self.hide_parts.iter().position(|y| *y == x).unwrap());
                        changed |= CHANGED_ALL;
                    }
                });
            }
        });

        ui.collapsing("Colors", |ui| {
            tooltip_helper(
                ui,
                |ui| {
                    ui.label("Information");
                },
                |ui| {
                    ui.label("The Color Slot system lets players change the colors of accessories. In this sub-menu you can select a node and any of it's materials, and assign a color slot to it.\n\
                    You can make multiple materials share the same slot. The slots also have a default color, you can use the \"Assign and Copy Color\" to set the default color to the current material, but you can also just change it by clicking on the color itself.\n\
                    The \"Preview Selected Material\" option makes the currently selected material slowly blink in a red color, letting you see which part of the model you currently have selected.");
                }
            );

            ui.checkbox(
                &mut self.material_flash,
                "Preview Selected Material",
            );

            let last_select = self.material_highlight_node_select;
            if let Some(obj) = &mut self.object {
                egui::ComboBox::from_label("Node Select")
                    .selected_text(if let Some(index) = self.material_highlight_node_select {
                        let mut counter = 0;
                        if let Some(selected) = obj.get_node(&mut counter, index) {
                            format!("{} ({})", selected.name, index)
                        } else {
                            "Invalid index".into()
                        }
                    } else {
                        "None".into()
                    })
                    .show_ui(ui, |ui| {
                        let node_count = obj.get_node_count();

                        for i in 0..node_count {
                            let mut counter = 0;
                            if let Some(node) = obj.get_node(&mut counter, i) {
                                if node.model.is_some() {
                                    ui.selectable_value(
                                        &mut self.material_highlight_node_select,
                                        Some(i),
                                        format!("{} ({})", node.name, i),
                                    );
                                }
                            }
                        }
                    });
            }
            if last_select != self.material_highlight_node_select {
                self.material_highlight_material_select = None;
            }

            if let Some(node_index) = self.material_highlight_node_select {
                egui::ComboBox::from_label("Material Select")
                    .selected_text(if let Some(select) = self.material_highlight_material_select {
                        format!("{}", select)
                    } else {
                        "None".to_string()
                    })
                    .show_ui(ui, |ui| {
                        if let Some(obj) = &mut self.object {
                            let mut counter = 0;
                            if let Some(poly_chunk_list) = obj
                                .get_node(&mut counter, node_index)
                                .and_then(|obj| obj.model.as_mut())
                                .and_then(|mdl| mdl.poly_list.as_mut())
                            {
                                let mut mat_counter = 0;
                                for p in poly_chunk_list {
                                    match p {
                                        PolyChunk::Material {
                                            source_alpha: _,
                                            destination_alpha: _,
                                            diffuse: _,
                                            ambient: _,
                                            specular: _,
                                        } => {
                                            ui.selectable_value(
                                                &mut self.material_highlight_material_select,
                                                Some(mat_counter),
                                                format!("{}", mat_counter),
                                            );

                                            mat_counter += 1;
                                        }
                                        _ => continue,
                                    }
                                }
                            }
                        }
                    });
            }

            if let Some(node) = self.material_highlight_node_select {
                if let Some(material) = self.material_highlight_material_select {
                    egui::ComboBox::from_label("Selected Slots")
                        .selected_text(format!("Slot {}", (self.selected_slot + 1)))
                        .show_ui(ui, |ui| {
                            for i in 0..self.material_slots.len() {
                                ui.selectable_value(
                                    &mut self.selected_slot,
                                    i,
                                    format!("Slot {}", (i + 1)),
                                );
                            }
                        });

                    if self.material_slot_users.contains_key(&(node, material)) {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "Using slot {}",
                                (self.material_slot_users[&(node, material)] + 1)
                            ));
                            if ui.button("Remove").clicked() {
                                self.material_slot_users.remove(&(node, material));

                                changed |= CHANGED_ALL;
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("Assign").clicked() {
                                self.material_slot_users
                                    .insert((node, material), self.selected_slot);

                                changed |= CHANGED_ALL;
                            }

                            if ui.button("Assign All").clicked() {
                                let mut material_count = 0;
                                if let Some(obj) = &mut self.object {
                                    let mut counter = 0;
                                    if let Some(poly_list) = obj
                                        .get_node(&mut counter, node)
                                        .and_then(|obj| obj.model.as_mut())
                                        .and_then(|mdl| mdl.poly_list.as_mut())
                                    {
                                        material_count = poly_list
                                            .iter()
                                            .filter(|poly| matches!(poly, PolyChunk::Material { source_alpha: _, destination_alpha: _, diffuse: _, ambient: _, specular: _ }))
                                            .count();
                                    }

                                    for i in 0..material_count {
                                        self.material_slot_users
                                            .insert((node, i), self.selected_slot);

                                        changed |= CHANGED_ALL;
                                    }
                                }
                            }
                        });

                        if ui.button("Assign and Copy Color").clicked() {
                            self.material_slot_users
                                .insert((node, material), self.selected_slot);
                            self.material_slots[self.selected_slot] =
                                self.material_backup_color[&(node, material)];

                            changed |= CHANGED_ALL;
                        }
                    }
                }
            }

            ui.collapsing("Slots", |ui| {
                for i in 0..self.material_slots.len() {
                    let slot = &mut self.material_slots[i];
                    let mut rgb = [
                        slot.r as f32 / 255.0,
                        slot.g as f32 / 255.0,
                        slot.b as f32 / 255.0,
                    ];
                    ui.horizontal(|ui| {
                        ui.label(format!("Slot {}", i + 1));

                        if ui.color_edit_button_rgb(&mut rgb).changed() {
                            slot.r = (rgb[0] * 255.0) as u8;
                            slot.g = (rgb[1] * 255.0) as u8;
                            slot.b = (rgb[2] * 255.0) as u8;

                            changed |= CHANGED_ALL;
                        }

                        ui.label(format!(
                            "(users: {})",
                            self.material_slot_users
                                .iter()
                                .filter(|(_, v)| **v == i)
                                .count()
                        ))
                    });
                }
            })
        });

        self.update_material_flash(ctx);

        if let Some(data) = &self.accessory_data {
            if let Some(json_path) = &self.json_path {
                ui.horizontal(|ui| {
                    if config.auto_save {
                        ui.colored_label(Color32::LIGHT_GREEN, "Autosave is enabled!");
                    } else if self.unsaved_changes {
                        ui.colored_label(Color32::RED, "Unsaved changes!");
                    }

                    if !self.id.is_empty() && ui.button("Save").clicked() {
                        let json_err = data.save_json(&self.market_data, json_path);
                        if json_err.is_err() {
                            show_error(format!("Failed to save json: {}", json_err.err().unwrap()));
                        } else {
                            self.unsaved_changes = false;
                        }
                    }
                });
            }
        }

        changed
    }

    pub fn update_material_flash(&mut self, ctx: &egui::Context) {
        let material_flicker = ctx.input(|i| (i.time * 5.0).sin()) as f32 * 0.5 + 0.5;

        let mut update_model = false;

        if let Some(obj) = &mut self.object {
            let node_count = obj.get_node_count();

            for i in 0..node_count {
                let mut counter = 0;
                if let Some(poly_chunk_list) = obj
                    .get_node(&mut counter, i)
                    .and_then(|obj| obj.model.as_mut())
                    .and_then(|mdl| mdl.poly_list.as_mut())
                {
                    let mut mat_counter = 0;
                    for p in poly_chunk_list {
                        match p {
                            PolyChunk::Material {
                                source_alpha: _,
                                destination_alpha: _,
                                diffuse,
                                ambient: _,
                                specular: _,
                            } => {
                                self.material_backup_color
                                    .entry((i, mat_counter))
                                    .or_insert_with(|| diffuse.unwrap());

                                let mut need_restore = true;
                                if self.material_slot_users.contains_key(&(i, mat_counter)) {
                                    *diffuse = Some(
                                        self.material_slots
                                            [self.material_slot_users[&(i, mat_counter)]],
                                    );
                                    need_restore = false;
                                    update_model = true;
                                }

                                if let Some(selected_node) = self.material_highlight_node_select {
                                    if let Some(selected_mat) =
                                        self.material_highlight_material_select
                                    {
                                        if self.material_flash
                                            && selected_node == i
                                            && selected_mat == mat_counter
                                        {
                                            *diffuse = Some(Color {
                                                r: (material_flicker * 255.0) as u8,
                                                g: 0,
                                                b: 0,
                                                a: 255,
                                            });

                                            need_restore = false;
                                            update_model = true;
                                        }
                                    }
                                }

                                if need_restore {
                                    *diffuse = Some(self.material_backup_color[&(i, mat_counter)]);
                                    update_model = true;
                                }

                                mat_counter += 1;
                            }
                            _ => continue,
                        }
                    }
                }
            }
        }

        if update_model {
            self.check_update();
        }
    }

    pub fn check_update(&mut self) {
        if self.disable_accessory_preview {
            self.chao_draw.chao.clear_accessory();
            self.chao_draw.chao.chao_shape.set_hide_head_parts(false);
            return;
        }

        if self.object.is_none() || self.texlist.is_none() {
            return;
        }

        let mut accessory_data = AccessoryData {
            id: self.id.clone(),
            object_path: self.object_path.clone(),
            texture_name: self.texture_name.clone(),
            object: self.object.as_ref().unwrap().clone(),
            texlist: self.texlist.as_ref().unwrap().clone(),
            accessory_type: self.accessory_type.clone(),
            use_renderfix: self.use_renderfix,
            renderfix_preview: self.renderfix_preview,
            hide_parts: self.hide_parts.clone(),
            disable_jiggle: self.disable_jiggle,
            bald_mode: self.bald_mode.clone(),
            bald_dont_hide_parts: self.bald_dont_hide_hparts,
            bald_preset_sides: self.bald_preset_sides,
            bald_center: self.bald_center,
            bald_clip_face: self.bald_clip_face,
            bald_influence: self.bald_influence,
            bald_radius: self.bald_radius,
            material_slot_users: self.material_slot_users.clone(),
            material_slots: self.material_slots,
        };

        if !accessory_data.check_if_generic_appropriate()
            && [AccessoryType::Generic1, AccessoryType::Generic2]
                .contains(&accessory_data.accessory_type)
        {
            self.accessory_type = AccessoryType::Head;
            accessory_data.accessory_type = self.accessory_type.clone();
        }

        self.accessory_data = Some(accessory_data);

        self.chao_draw
            .chao
            .chao_shape
            .set_hide_head_parts(self.bald_mode != BaldMode::None && !self.bald_dont_hide_hparts);

        self.chao_draw
            .chao
            .set_accessory(self.accessory_data.as_mut().unwrap());
    }

    pub fn new(chao_global_state: &Rc<ChaoGlobalState>, open_or_save: bool) -> Self {
        Self {
            disable_accessory_preview: false,
            hide_parts_selected_num: 0,
            id: "".to_string(),
            open_or_save,
            json_path: None,
            chao_draw: ChaoDraw::new(
                chao_global_state,
                &PathBuf::from_str("res/test.chao").unwrap(),
            ),
            object: None,
            object_path: None,
            texlist: None,
            texture_name: None,
            accessory_data: None,
            accessory_type: AccessoryType::Head,
            hide_parts: Vec::new(),
            disable_jiggle: false,
            market_data: MarketData::default(),
            bald_mode: BaldMode::None,
            bald_dont_hide_hparts: false,
            bald_preset_sides: [false, false, false],
            bald_center: BALD_DEFAULT_CENTER,
            bald_clip_face: BALD_DEFAULT_CLIP_FACE,
            bald_influence: glam::vec3(
                BALD_DEFAULT_INFLUENCE,
                BALD_DEFAULT_INFLUENCE,
                BALD_DEFAULT_INFLUENCE,
            ),
            bald_radius: BALD_DEFAULT_RADIUS,

            use_renderfix: true,
            renderfix_preview: true,

            selected_slot: 0,
            material_slot_users: HashMap::new(),
            material_slots: [Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            }; 8],
            material_backup_color: HashMap::new(),
            material_flash: false,
            material_highlight_node_select: None,
            material_highlight_material_select: None,

            unsaved_changes: false,
        }
    }
}
