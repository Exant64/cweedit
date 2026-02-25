use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
    rc::Rc,
};

use egui_wgpu::RenderState;

use crate::ninja::{
    anim::NinjaMotion,
    error::NinjaParseError,
    modelfile::NinjaChunkObject,
    texlist::NinjaGpuTexEntry,
    texture::{
        error::NinjaTexReadError,
        gvm::NinjaTexlist,
        palette::{NinjaPalette, NinjaPaletteError},
    },
};

pub struct ChaoGlobalState {
    pub al_body_texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    pub al_eye_texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    pub al_mouth_texlist: Rc<NinjaTexlist<NinjaGpuTexEntry, RenderState>>,
    pub root_objects: Vec<NinjaChunkObject>,
    pub masks: Vec<NinjaChunkObject>,
    pub palettes: [NinjaPalette; 39],
    pub stand_anim: NinjaMotion,
    pub crawl_anim: NinjaMotion,
    pub sitting_anim: NinjaMotion,
    pub running_anim: NinjaMotion,
}

impl ChaoGlobalState {
    fn error_dialog(desc: &String) {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .set_title("Error initializing the editor!")
            .set_description(desc)
            .show();

        panic!();
    }

    fn load_model(filename: &str) -> NinjaChunkObject {
        let mut f = File::open(filename)
            .inspect_err(|_| Self::error_dialog(&format!("Failed to open {}!", filename)))
            .unwrap();

        let metadata = fs::metadata(filename)
            .inspect_err(|_| {
                Self::error_dialog(&format!("Failed to read metadata for {}!", filename))
            })
            .unwrap();

        let mut buffer = vec![0; metadata.len() as usize];
        let _ = f.read(&mut buffer).inspect_err(|_| {
            Self::error_dialog(&format!("Failed to read content of {}!", filename))
        });

        NinjaChunkObject::new(buffer.as_slice())
            .map_err(|e| match e {
                NinjaParseError::FormatError(str) => {
                    Self::error_dialog(&format!("Failed to load {}: {}", filename, str))
                }
                NinjaParseError::IoError() => {
                    Self::error_dialog(&format!("Failed to load {}: IO error", filename))
                }
            })
            .unwrap()
    }

    fn load_masks() -> Vec<NinjaChunkObject> {
        let mut stewpan = Self::load_model("res/masks/object_al_mask_stewpan_mask_stewpan.sa2mdl");
        let mut stewpan_eye =
            Self::load_model("res/masks/object_al_mask_stewpan_eye_mask_stewpan_eye.sa2mdl");
        stewpan_eye.sibling = Some(Box::from(Self::load_model(
            "res/masks/object_al_mask_stewpan_jaw_mask_stewpan_jaw.sa2mdl",
        )));
        stewpan.sibling = Some(Box::from(stewpan_eye));

        vec![
            Self::load_model("res/masks/object_al_pumpkinhead_pumpkinhead.sa2mdl"),
            Self::load_model("res/masks/object_al_skullhead_skullhead.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_apple_mask_apple.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_bucket_mask_bucket.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_can_mask_can.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_cdbox_mask_cdbox.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_flowerpot_mask_flowerpot.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_paperbag_mask_paperbag.sa2mdl"),
            stewpan,
            Self::load_model("res/masks/object_al_mask_stump_mask_stump.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_wmelon_mask_wmelon.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_wool_a_mask_wool_a.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_wool_b_mask_wool_b.sa2mdl"),
            Self::load_model("res/masks/object_al_mask_wool_c_mask_wool_c.sa2mdl"),
            Self::load_model("res/masks/object_al_mzsk_teethingring_mzsk_teethingring.sa2mdl"),
        ]
    }

    pub fn init(render_state: &RenderState) -> Self {
        let load_tex = |path: &'static str| {
            NinjaTexlist::load_tex(render_state, &PathBuf::from(path))
                .map_err(|e| match e {
                    NinjaTexReadError::FormatError(str) => {
                        Self::error_dialog(&format!("Failed to load {}: {}", path, str))
                    }
                    NinjaTexReadError::IoError() => {
                        Self::error_dialog(&format!("Failed to load {}: IO error", path))
                    }
                })
                .unwrap()
        };

        let load_pal = |path: &'static str| {
            NinjaPalette::read_gvp(&PathBuf::from(path))
                .map_err(|e| match e {
                    NinjaPaletteError::FormatError(str) => {
                        Self::error_dialog(&format!("Failed to load {}: {}", path, str))
                    }
                    NinjaPaletteError::IoError() => {
                        Self::error_dialog(&format!("Failed to load {}: IO error", path))
                    }
                })
                .unwrap()
        };

        let load_mot = |path: &'static str| {
            let vec = std::fs::read(path)
                .inspect_err(|_| Self::error_dialog(&format!("Failed to read {}!", path)))
                .unwrap();
            NinjaMotion::new(&vec)
                .inspect_err(|e| match e {
                    NinjaParseError::FormatError(str) => {
                        Self::error_dialog(&format!("Failed to load {}: {}", path, str))
                    }
                    NinjaParseError::IoError() => {
                        Self::error_dialog(&format!("Failed to load {}: IO error", path))
                    }
                })
                .unwrap()
        };

        let mut al_body_texlist = load_tex("res/al_body.gvm");
        let al_eye_texlist = load_tex("res/al_eye.gvm");
        let al_mouth_texlist = load_tex("res/al_mouth.gvm");

        for i in (0..18).step_by(3) {
            al_body_texlist.textures[i].bank = 0;
            al_body_texlist.textures[i + 1].bank = 1;
            al_body_texlist.textures[i + 2].bank = 2;
        }

        let palettes = [
            load_pal("res/palette/AL_NC00.GVP"),
            load_pal("res/palette/AL_NC01.GVP"),
            load_pal("res/palette/AL_HCZ.GVP"),
            load_pal("res/palette/AL_HCN.GVP"),
            load_pal("res/palette/AL_HCS.GVP"),
            load_pal("res/palette/AL_HCF.GVP"),
            load_pal("res/palette/AL_HCR.GVP"),
            load_pal("res/palette/AL_HCP.GVP"),
            load_pal("res/palette/AL_DC.GVP"),
            load_pal("res/palette/AL_HNZ.GVP"),
            load_pal("res/palette/AL_HNN.GVP"),
            load_pal("res/palette/AL_HNS.GVP"),
            load_pal("res/palette/AL_HNF.GVP"),
            load_pal("res/palette/AL_HNR.GVP"),
            load_pal("res/palette/AL_HNP.GVP"),
            load_pal("res/palette/AL_HSZ.GVP"),
            load_pal("res/palette/AL_HSN.GVP"),
            load_pal("res/palette/AL_HSS.GVP"),
            load_pal("res/palette/AL_HSF.GVP"),
            load_pal("res/palette/AL_HSR.GVP"),
            load_pal("res/palette/AL_HSP.GVP"),
            load_pal("res/palette/AL_HFZ.GVP"),
            load_pal("res/palette/AL_HFN.GVP"),
            load_pal("res/palette/AL_HFS.GVP"),
            load_pal("res/palette/AL_HFF.GVP"),
            load_pal("res/palette/AL_HFR.GVP"),
            load_pal("res/palette/AL_HFP.GVP"),
            load_pal("res/palette/AL_HRZ.GVP"),
            load_pal("res/palette/AL_HRN.GVP"),
            load_pal("res/palette/AL_HRS.GVP"),
            load_pal("res/palette/AL_HRF.GVP"),
            load_pal("res/palette/AL_HRR.GVP"),
            load_pal("res/palette/AL_HRP.GVP"),
            load_pal("res/palette/AL_HPZ.GVP"),
            load_pal("res/palette/AL_HPN.GVP"),
            load_pal("res/palette/AL_HPS.GVP"),
            load_pal("res/palette/AL_HPF.GVP"),
            load_pal("res/palette/AL_HPR.GVP"),
            load_pal("res/palette/AL_HPP.GVP"),
        ];

        let mut models = Vec::new();

        for i in 0..108 {
            models.push(Self::load_model(
                format!("res/al_rootobject/{i}.sa2mdl").as_str(),
            ));
        }

        for root_obj_index in [108, 114, 120, 126, 132, 138] {
            let model =
                Self::load_model(format!("res/al_rootobject/{root_obj_index}.sa2mdl").as_str());
            for _ in 0..6 {
                models.push(model.clone());
            }
        }

        ChaoGlobalState {
            al_body_texlist: Rc::new(al_body_texlist),
            al_eye_texlist: Rc::new(al_eye_texlist),
            al_mouth_texlist: Rc::new(al_mouth_texlist),
            palettes,
            masks: Self::load_masks(),
            root_objects: models,
            stand_anim: load_mot("res/chaomotion/0.saanim"),
            crawl_anim: load_mot("res/chaomotion/110.saanim"),
            sitting_anim: load_mot("res/chaomotion/62.saanim"),
            running_anim: load_mot("res/chaomotion/119.saanim"),
        }
    }
}
