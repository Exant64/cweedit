use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use egui::Ui;

use crate::{
    chao::Chao,
    chaoparam::*,
    chaostate::ChaoGlobalState,
    ninja::{anim::NinjaMotion, ninjadraw::NinjaState},
};

use super::Drawable;

pub struct ChaoDraw {
    pub chao: Chao,
    pub motions: [NinjaMotion; 4],
    pub frame: f32,
    anim_playing: bool,
    anim_speeds: [f32; 4],
    selected_anim: Option<usize>,
    is_draw_enabled: bool,
}

impl Drawable for ChaoDraw {
    fn draw(&mut self, device: &wgpu::Device, ninja: &Rc<RefCell<NinjaState>>) {
        if !self.is_draw_enabled {
            return;
        }

        if let Some(index) = self.selected_anim {
            self.chao
                .render(Some((&self.motions[index], self.frame)), device, ninja);
        } else {
            self.chao.render(None, device, ninja);
        }
    }
}

impl ChaoDraw {
    pub fn refresh_every_frame(&self) -> bool {
        self.anim_playing
    }

    pub fn chao_preview_edit(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        const CHAO_TYPE_NAME_PAIRS: [(&str, i8); 22] = [
            ("Child", TYPE_CHILD),
            ("Neutral Normal", TYPE_N_NORMAL),
            ("Hero Normal", TYPE_H_NORMAL),
            ("Dark Normal", TYPE_D_NORMAL),
            ("Neutral Swim", TYPE_N_SWIM),
            ("Hero Swim", TYPE_H_SWIM),
            ("Dark Swim", TYPE_D_SWIM),
            ("Neutral Fly", TYPE_N_FLY),
            ("Hero Fly", TYPE_H_FLY),
            ("Dark Fly", TYPE_D_FLY),
            ("Neutral Run", TYPE_N_RUN),
            ("Hero Run", TYPE_H_RUN),
            ("Dark Run", TYPE_D_RUN),
            ("Neutral Power", TYPE_N_POWER),
            ("Hero Power", TYPE_H_POWER),
            ("Dark Power", TYPE_D_POWER),
            ("Neutral Chaos", TYPE_N_CHAOS),
            ("Hero Chaos", TYPE_H_CHAOS),
            ("Dark Chaos", TYPE_D_CHAOS),
            ("Tails", TYPE_TAILS),
            ("Knuckles", TYPE_KNUCKLES),
            ("Amy", TYPE_AMY),
        ];

        let chao_name_hashmap: HashMap<i8, &str> = CHAO_TYPE_NAME_PAIRS
            .into_iter()
            .map(|(a, b)| (b, a))
            .collect();

        ui.add(egui::Checkbox::new(
            &mut self.is_draw_enabled,
            "Preview enabled",
        ));

        let mut chao_type = self.chao.get_type();
        egui::ComboBox::from_label("Chao Type")
            .selected_text(chao_name_hashmap[&chao_type])
            .show_ui(ui, |ui| {
                for (name, value) in CHAO_TYPE_NAME_PAIRS {
                    ui.selectable_value(&mut chao_type, value, name);
                }
            });

        if chao_type != self.chao.get_type() {
            self.chao.update_type(chao_type);
        }

        let body_info = self.chao.get_body_info();
        ui.add(egui::Slider::new(&mut body_info.h_pos, -1.0..=1.0).text("Swim-Fly"));
        ui.add(egui::Slider::new(&mut body_info.v_pos, -1.0..=1.0).text("Run-Power"));
        ui.add(egui::Slider::new(&mut body_info.growth, 0.0..=1.2).text("Magnitude"));

        if chao_type == TYPE_CHILD {
            ui.add(egui::Slider::new(&mut body_info.a_pos, -1.0..=1.0).text("Alignment"));
        }

        let mut is_monotone = body_info.non_tex != 0;
        ui.add(egui::Checkbox::new(&mut is_monotone, "Monotone"));
        body_info.non_tex = is_monotone as i8;

        let mut is_shiny = body_info.multi_num != 0;
        ui.add(egui::Checkbox::new(&mut is_shiny, "Shiny"));
        body_info.multi_num = is_shiny as i8;

        const ANIMATION_NAMES: [&str; 4] = ["Standing", "Crawling", "Sitting", "Running"];

        let prev_selected_anim = self.selected_anim;
        egui::ComboBox::from_label("Animation")
            .selected_text(if let Some(index) = self.selected_anim {
                ANIMATION_NAMES[index]
            } else {
                "None"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_anim, None, "None");
                for (index, name) in ANIMATION_NAMES.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_anim, Some(index), *name);
                }
            });

        if prev_selected_anim != self.selected_anim {
            self.frame = 0.0;
            self.anim_playing = false;
        }

        if let Some(index) = self.selected_anim {
            let num_frames = self.motions[index].get_num_frames() as f32 - 1.0;

            if self.anim_playing {
                ctx.input(|i| {
                    self.frame += (self.anim_speeds[index] * 60.0) * i.stable_dt;
                    if self.frame >= num_frames {
                        self.frame = 0.0;
                    }
                });
            }

            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut self.frame, 0.0..=num_frames));
                let button_text = if self.anim_playing { "⏸" } else { "▶" };

                if ui.button(button_text).clicked() {
                    self.anim_playing = !self.anim_playing;
                }
            });
        }

        self.chao.update();
    }

    pub fn new(chao_global_state: &Rc<ChaoGlobalState>, _chao_path: &Path) -> Self {
        Self {
            chao: Chao::create(chao_global_state, &ChaoParamGc::default()),
            motions: [
                chao_global_state.stand_anim.clone(),
                chao_global_state.crawl_anim.clone(),
                chao_global_state.sitting_anim.clone(),
                chao_global_state.running_anim.clone(),
            ],
            selected_anim: None,
            frame: 0.0,
            anim_playing: false,
            is_draw_enabled: true,
            anim_speeds: [0.12, 0.07, 0.12, 0.21],
        }
    }
}
