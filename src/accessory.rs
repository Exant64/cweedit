use crate::chao::{
    BALD_DEFAULT_CENTER, BALD_DEFAULT_CLIP_FACE, BALD_DEFAULT_INFLUENCE, BALD_DEFAULT_RADIUS,
};
use crate::genericjson::MarketData;
use crate::ninja::math::Color;
use crate::ninja::modelfile::NinjaChunkObject;
use crate::ninja::texlist::NinjaGpuTexEntry;
use crate::ninja::texture::gvm::NinjaTexlist;
use egui_wgpu::RenderState;
use hex_color::HexColor;
use rfd::{FileDialog, MessageDialogResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

#[derive(PartialEq, Debug, Clone)]
pub enum AccessoryType {
    Head,
    Face,
    Generic1,
    Generic2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BaldMode {
    None,
    Presets,
    Custom,
}

#[derive(Clone)]
pub struct AccessoryData {
    pub id: String,
    pub object_path: Option<PathBuf>,
    pub texture_name: Option<String>,
    pub object: Box<NinjaChunkObject>,
    pub texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    pub accessory_type: AccessoryType,
    pub hide_parts: Vec<u8>,
    pub disable_jiggle: bool,
    pub bald_mode: BaldMode,
    pub bald_dont_hide_parts: bool,
    pub bald_preset_sides: [bool; 3],
    pub bald_influence: glam::Vec3,
    pub bald_center: glam::Vec3,
    pub bald_radius: f32,
    pub bald_clip_face: bool,

    pub use_renderfix: bool,
    pub renderfix_preview: bool,

    pub material_slot_users: HashMap<(usize, usize), usize>,
    pub material_slots: [Color; 8],
}

impl AccessoryData {
    pub fn check_if_generic_appropriate(&self) -> bool {
        self.object.get_node_count() == 40
    }

    pub fn check_renderfix_render(&self) -> bool {
        self.renderfix_preview
    }

    fn safety_check_before_save(&self, json_path: &PathBuf) -> std::result::Result<&str, String> {
        let relative_object_path = if let Some(obj_path) = &self.object_path {
            obj_path
                .as_path()
                .strip_prefix(
                    json_path
                        .parent()
                        .ok_or("Failed to retrieve parent of path!")?,
                )
                .map_err(|_| "Object path isn't starting from CWE/Accessories")
        } else {
            Err("object_path is empty!")
        }?
        .to_str()
        .ok_or("Failed to convert final model path to string!")?;

        if self.id.is_empty() {
            return Err("ID cannot be empty!".into());
        }

        Ok(relative_object_path)
    }

    pub fn save_json(
        &self,
        market_data: &MarketData,
        json_path: &PathBuf,
    ) -> std::result::Result<(), String> {
        let relative_object_path = self.safety_check_before_save(json_path)?;

        let json_colors = self
            .material_slots
            .map(|x| hex_color::Display::new(HexColor::rgb(x.r, x.g, x.b)).to_string());
        let used_slots: Vec<usize> = (0..8)
            .filter(|x| {
                self.material_slot_users
                    .clone()
                    .into_values()
                    .any(|v| v == *x)
            })
            .collect();
        let entries: Vec<Value> = self
            .material_slot_users
            .iter()
            .map(|((node, mat), slot)| {
                json!({
                    "node_index": *node,
                    "material_index": *mat,
                    "slot_index": *slot
                })
            })
            .collect();

        let mut contents = json!({
            "id": self.id,
            "model": relative_object_path,
            "texture": self.texture_name.as_ref().ok_or("Texture name is empty!")?,
            "slot": match self.accessory_type {
                AccessoryType::Head => "head",
                AccessoryType::Face => "face",
                AccessoryType::Generic1 => "generic1",
                AccessoryType::Generic2 => "generic2",
            },
            "renderfix": self.use_renderfix,
            "bald_dont_hide_parts": self.bald_dont_hide_parts,
            "hide_parts": self.hide_parts,
            "disable_jiggle": self.disable_jiggle,
            "market_data": {
                "price": market_data.price,
                "sale": market_data.sale,
                "name": market_data.name,
                "description": market_data.description,
                "emblems": market_data.emblems,
            },
            "color_slots": {
                "used": used_slots,
                "colors": json_colors,
                "entries": entries
            }
        });

        match self.bald_mode {
            BaldMode::None => contents["bald_mode"] = "none".into(),
            BaldMode::Presets => contents["bald_mode"] = self.bald_preset_sides.into(),
            BaldMode::Custom => {
                contents["bald_mode"] = json!({
                    "influence": [self.bald_influence.x, self.bald_influence.y, self.bald_influence.z],
                    "center": [self.bald_center.x, self.bald_center.y, self.bald_center.z],
                    "radius": self.bald_radius,
                    "clip_face": self.bald_clip_face
                });
            }
        }

        std::fs::write(json_path, contents.to_string()).map_err(|_| "Failed to save JSON!")?;

        Ok(())
    }

    pub fn read_json(
        _: &egui::Context,
        frame: &eframe::Frame,
        json_path: &PathBuf,
    ) -> std::result::Result<(Self, MarketData), String> {
        let path = json_path.as_path();
        let parent = path
            .parent()
            .ok_or("Couldn't retrieve parent path of json file".to_string())?;
        if !parent.ends_with("CWE/Accessories") {
            return Err("JSON file not in \"CWE/Accessories\" folder!".to_string());
        }

        let document_text =
            std::fs::read_to_string(path).or(Err("Failed to read JSON file!".to_string()))?;
        let document: Value =
            serde_json::from_str(document_text.as_str()).map_err(|f| f.to_string())?;

        let id = if let Some(id) = document["id"].as_str() {
            if id.len() > 20 {
                return Err("id is longer than 20!".to_string());
            }

            Ok(id.to_string())
        } else {
            Err("id is not a string!".to_string())
        }?;

        let (object, object_path) = if let Some(path) = document["model"].as_str() {
            //if !PathBuf::from(path).as_path().starts_with(parent) {
            //    return Err("Model file not in CWE/Accessories folder!".to_string());
            //}
            let obj_path = parent.join(path);
            Ok((
                NinjaChunkObject::read_file(&obj_path).map_err(|_| "Invalid chunk object!")?,
                obj_path,
            ))
        } else {
            Err("Couldn't retrieve specified object!")
        }?;

        let (texlist, texture_path) = if let Some(path) = document["texture"].as_str() {
            let file_name = format!("Find {} texture file", path);

            let mut final_path = path.to_string();
            if let Some(pathbuf) = FileDialog::new()
                .set_title(&file_name)
                .add_filter(file_name, &["pak"])
                .pick_file()
            {
                if let Some(stem_str) = pathbuf.file_stem().and_then(|stem| stem.to_str()) {
                    if stem_str != path {
                        let diag_result = rfd::MessageDialog::new()
                            .set_buttons(rfd::MessageButtons::YesNo)
                            .set_title("Loading Accessory")
                            .set_description("Texture file does not match file name specified in json! Do you want to change the specified texture to the selected one?")
                            .show();

                        if diag_result == MessageDialogResult::No {
                            return Err("".into());
                        }

                        final_path = stem_str.to_string();
                    }
                }

                Ok((
                    NinjaTexlist::load_tex(
                        frame
                            .wgpu_render_state()
                            .ok_or("Failed to get wgpu_render_state!")?,
                        &pathbuf,
                    )
                    .map_err(|e| format!("Texture failed to load! {:#?}", e))?
                    .into(),
                    final_path,
                ))
            } else {
                Err("Texture not chosen!")
            }
        } else {
            Err("No texture specified!")
        }?;

        let mut bald_preset_sides = [false, false, false];
        let mut bald_influence = glam::Vec3::new(
            BALD_DEFAULT_INFLUENCE,
            BALD_DEFAULT_INFLUENCE,
            BALD_DEFAULT_INFLUENCE,
        );
        let mut bald_center = BALD_DEFAULT_CENTER;
        let mut bald_radius = BALD_DEFAULT_RADIUS;
        let mut bald_clip_face = BALD_DEFAULT_CLIP_FACE;

        let mut use_renderfix = false;

        if let Some(b) = document["renderfix"].as_bool() {
            use_renderfix = b;
        }

        let bald_mode = if let Some(arr) = document["bald_mode"].as_array() {
            if arr.len() != 3 {
                Err("presets bald_mode not an array of three elements".to_string())
            } else {
                bald_preset_sides = [
                    arr[0].as_bool().ok_or("presets bald_mode not a boolean!")?,
                    arr[1].as_bool().ok_or("presets bald_mode not a boolean!")?,
                    arr[2].as_bool().ok_or("presets bald_mode not a boolean!")?,
                ];

                Ok(BaldMode::Presets)
            }
        } else if let Some(obj) = document["bald_mode"].as_object() {
            bald_influence = if let Some(inf) = obj["influence"].as_array() {
                let arr = glam::vec3(
                    inf[0]
                        .as_f64()
                        .ok_or("bald influence element not a float!")? as f32,
                    inf[1]
                        .as_f64()
                        .ok_or("bald influence element not a float!")? as f32,
                    inf[2]
                        .as_f64()
                        .ok_or("bald influence element not a float!")? as f32,
                );
                Ok(arr)
            } else {
                Err("bald influence not an array!")
            }?;

            bald_center = if let Some(inf) = obj["center"].as_array() {
                let arr = glam::vec3(
                    inf[0].as_f64().ok_or("bald center element not a float!")? as f32,
                    inf[1].as_f64().ok_or("bald center element not a float!")? as f32,
                    inf[2].as_f64().ok_or("bald center element not a float!")? as f32,
                );
                Ok(arr)
            } else {
                Err("bald center not an array!")
            }?;

            bald_radius = if let Some(flt) = obj["radius"].as_f64() {
                Ok(flt as f32)
            } else {
                Err("bald radius not a float!")
            }?;

            bald_clip_face = if let Some(clip) = obj["clip_face"].as_bool() {
                Ok(clip)
            } else {
                Err("bald clip_face not a bool!")
            }?;

            Ok(BaldMode::Custom)
        } else {
            Ok(BaldMode::None)
        }?;

        let hide_parts = if let Some(array) = document["hide_parts"].as_array() {
            let mut v = Vec::new();
            for x in array {
                v.push(x.as_i64().ok_or("hide_parts element is not an integer!")? as u8);
            }
            Ok(v)
        } else {
            Err("hide_parts isn't a valid array!")
        }?;

        let accessory_type = if let Some(acc_type) = document["slot"].as_str() {
            match acc_type {
                "head" => Ok(AccessoryType::Head),
                "face" => Ok(AccessoryType::Face),
                "generic1" => Ok(AccessoryType::Generic1),
                "generic2" => Ok(AccessoryType::Generic2),
                _ => Err("slot isn't a valid slot!"),
            }
        } else {
            Err("slot isn't a string!")
        }?;

        let disable_jiggle = if let Some(b) = document["disable_jiggle"].as_bool() {
            Ok(b)
        } else {
            Err("disable_jiggle isn't a boolean!")
        }?;

        let bald_dont_hide_parts = document["bald_dont_hide_parts"].as_bool().unwrap_or_default();

        let market_data = MarketData::read_json(&document)?;

        let mut material_slot_users: HashMap<(usize, usize), usize> = HashMap::new();
        let mut material_slots: [Color; 8] = [Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }; 8];

        if let Some(color_slot_data) = document["color_slots"].as_object() {
            if let Some(entries) = color_slot_data["entries"].as_array() {
                for entry in entries {
                    if !entry.is_object() {
                        return Err("color_slots.entries entry is not an object!".into());
                    }

                    let node_index = if let Some(node) = entry["node_index"].as_i64() {
                        Ok(node as usize)
                    } else {
                        Err("color_slots.entries.node_index is not an integer!")
                    }?;

                    let material_index = if let Some(mat) = entry["material_index"].as_i64() {
                        Ok(mat as usize)
                    } else {
                        Err("color_slots.entries.material_index is not an integer!")
                    }?;

                    let slot_index = if let Some(slot) = entry["slot_index"].as_i64() {
                        Ok(slot as usize)
                    } else {
                        Err("color_slots.entries.slot_index is not an integer!")
                    }?;

                    material_slot_users.insert((node_index, material_index), slot_index);
                }
            } else {
                return Err("color_slots.entries not an array!".into());
            }

            if let Some(array) = color_slot_data["colors"].as_array() {
                if array.len() != material_slots.len() {
                    return Err(
                        "color_slots.colors array length does not match requirement!".into(),
                    );
                }

                for i in 0..array.len() {
                    if let Some(str) = array[i].as_str() {
                        let color =
                            HexColor::from_str(str).map_err(|_| "failed to parse hex color!")?;

                        material_slots[i] = Color {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                            a: 255,
                        };

                        continue;
                    }

                    return Err("color_slots.colors elements are not hex color strings!".into());
                }
            } else {
                return Err("color_slots.colors is not an array!".into());
            }
        } else if !document["color_slots"].is_null() {
            return Err("color_slots is not an object!".into());
        }

        Ok((
            AccessoryData {
                id,
                object,
                object_path: Some(object_path),
                texture_name: Some(texture_path.to_string()),
                texlist,
                accessory_type,
                hide_parts,
                disable_jiggle,
                use_renderfix,
                renderfix_preview: true,
                bald_mode,
                bald_dont_hide_parts,
                bald_preset_sides,
                bald_influence,
                bald_center,
                bald_clip_face,
                bald_radius,
                material_slot_users,
                material_slots,
            },
            market_data,
        ))
    }
}
