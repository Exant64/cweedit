use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use glam::Mat4;

use crate::{
    accessory::{AccessoryData, AccessoryType, BaldMode},
    chaoface::ChaoFace,
    chaoparam::{ChaoBodyInfo, ChaoParamGc},
    chaoshape::ChaoShape,
    chaostate::ChaoGlobalState,
    ninja::{
        anim::NinjaMotion, chunkmodel::ChunkModel, math::Point3, modelfile::NinjaChunkObject,
        polychunk::PolyChunk,
    },
    NinjaRotation, NinjaState,
};

pub const BALD_DEFAULT_INFLUENCE: f32 = 0.9;
pub const BALD_DEFAULT_CENTER: glam::Vec3 = glam::Vec3::new(0.0, 1.1, -0.01);
pub const BALD_DEFAULT_RADIUS: f32 = 1.15;
pub const BALD_DEFAULT_CLIP_FACE: bool = true;

struct ChaoDraw {
    node_index: usize,
    slope_ang: i32,
    close_ang: i32,
    eye_tex_id: u16,
    mouth_tex_id: [u16; 2],
    node_matrices: [glam::Mat4; 40],
}

impl ChaoDraw {
    fn set_chao_mode(&self, chao_param: &ChaoParamGc, ninja_state: &mut RefMut<'_, NinjaState>) {
        ninja_state.set_chao_mode(0, 0);
        if chao_param.body.jewel_num > 0 {
            ninja_state.set_chao_mode(1, 17 + chao_param.body.jewel_num as usize);
        } else if chao_param.body.multi_num > 0 {
            if chao_param.body.non_tex > 0 {
                ninja_state.set_chao_mode(3, 34);
            } else {
                ninja_state.set_chao_mode(4, 34);
            }
        } else if chao_param.body.non_tex > 0 {
            ninja_state.set_chao_mode(2, 0);
        }
    }

    fn set_model_texid(chunk_model: &mut ChunkModel, texid: &[u16]) {
        let model = chunk_model.poly_list.as_mut().unwrap();
        let iter = model.iter_mut();
        let textures = iter.filter(|x| {
            matches!(
                x,
                PolyChunk::TinyTextureID {
                    mipmap_d_adjust: _,
                    clamp_u: _,
                    clamp_v: _,
                    flip_u: _,
                    flip_v: _,
                    texture_id: _,
                    super_sample: _,
                    filter_mode: _
                }
            )
        });

        textures.zip(texid.iter()).for_each(|(x, y)| {
            if let PolyChunk::TinyTextureID {
                mipmap_d_adjust: _,
                clamp_u: _,
                clamp_v: _,
                flip_u: _,
                flip_v: _,
                texture_id,
                super_sample: _,
                filter_mode: _,
            } = x
            {
                *texture_id = *y;
            }
        });
    }

    pub fn draw_chao(
        &mut self,
        chao_param: &ChaoParamGc,
        device: &wgpu::Device,
        ninja_state: &Rc<RefCell<NinjaState>>,
        chao_global_state: &Rc<ChaoGlobalState>,
        obj: &mut Box<NinjaChunkObject>,
        diff: &Vec<Point3>,
        motion: Option<(&NinjaMotion, f32)>,
        accessory: &Option<AccessoryData>,
    ) {
        {
            let mut ninja = ninja_state.borrow_mut();

            let mut can_draw = true;

            ninja.matrix_stack.push();

            if let Some((motion, frame)) = motion {
                if let Some(mot_pos) = motion.get_motion_pos(self.node_index, frame) {
                    ninja
                        .matrix_stack
                        .translate(mot_pos.x, mot_pos.y, mot_pos.z);
                    ninja.matrix_stack.translate(
                        diff[self.node_index].x,
                        diff[self.node_index].y,
                        diff[self.node_index].z,
                    );
                } else {
                    ninja
                        .matrix_stack
                        .translate(obj.pos.x, obj.pos.y, obj.pos.z);
                }

                if let Some(mot_ang) = motion.get_motion_ang(self.node_index, frame) {
                    ninja.matrix_stack.rotate(&mot_ang);
                } else {
                    ninja.matrix_stack.rotate(&obj.ang);
                }

                if let Some(mot_scl) = motion.get_motion_scl(self.node_index, frame) {
                    ninja.matrix_stack.scale(mot_scl.x, mot_scl.y, mot_scl.z);
                } else {
                    ninja.matrix_stack.scale(obj.scl.x, obj.scl.y, obj.scl.z);
                }
            } else {
                ninja
                    .matrix_stack
                    .translate(obj.pos.x, obj.pos.y, obj.pos.z);
                ninja.matrix_stack.rotate(&obj.ang);
                ninja.matrix_stack.scale(obj.scl.x, obj.scl.y, obj.scl.z);
            }

            self.node_matrices[self.node_index] = ninja.matrix_stack.get();

            if chao_param.body.obake_head > 0 {
                if self.node_index == 16 {
                    ninja.draw(
                        device,
                        &chao_global_state.masks[(chao_param.body.obake_head - 1) as usize],
                        chao_global_state.al_body_texlist.clone(),
                    );
                }

                can_draw = !match self.node_index {
                    16 => true,
                    18 => true,
                    21 => true,
                    19 => true,
                    22 => true,
                    23 => true,
                    25 => true,
                    24 => true,
                    26 => true,
                    30 => true,
                    31 => true,
                    27 => true,
                    29 => true,
                    _ => false,
                };
            }

            if let Some(accessory) = accessory {
                let node = self.node_index as u8;
                if self.node_index == 23 || self.node_index == 25 {
                    if accessory.hide_parts.contains(&node) {
                        can_draw = false;
                    } else {
                        can_draw &=
                            accessory.bald_mode == BaldMode::None || accessory.bald_dont_hide_parts;
                    }
                } else {
                    can_draw &= !accessory.hide_parts.contains(&node);
                }
            }

            if can_draw {
                if let Some(ref mut mdl) = obj.model {
                    match self.node_index {
                        18 | 21 => {
                            Self::set_model_texid(mdl, &[self.eye_tex_id]);
                            ninja.draw_mdl(device, mdl, chao_global_state.al_eye_texlist.clone());
                        }
                        27 => {
                            Self::set_model_texid(mdl, &self.mouth_tex_id);
                            ninja.draw_mdl(device, mdl, chao_global_state.al_mouth_texlist.clone());
                        }
                        19 | 22 => {
                            if self.close_ang != -16384 {
                                // eyelid
                                if self.node_index == 19 {
                                    ninja.matrix_stack.rotate(&NinjaRotation {
                                        x: self.close_ang,
                                        y: 0,
                                        z: self.slope_ang,
                                    });
                                } else if self.node_index == 22 {
                                    ninja.matrix_stack.rotate(&NinjaRotation {
                                        x: self.close_ang,
                                        y: 0,
                                        z: -self.slope_ang,
                                    });
                                }
                                self.set_chao_mode(chao_param, &mut ninja);
                                ninja.draw_mdl(
                                    device,
                                    mdl,
                                    chao_global_state.al_body_texlist.clone(),
                                );
                            }
                        }
                        _ => {
                            if let Some(accessory) = accessory {
                                if self.node_index == 16 && accessory.bald_mode != BaldMode::None {
                                    let influence = match accessory.bald_mode {
                                        BaldMode::Custom => accessory.bald_influence,
                                        BaldMode::Presets => glam::vec3(
                                            BALD_DEFAULT_INFLUENCE
                                                * accessory.bald_preset_sides[0] as i32 as f32,
                                            BALD_DEFAULT_INFLUENCE
                                                * accessory.bald_preset_sides[1] as i32 as f32,
                                            BALD_DEFAULT_INFLUENCE
                                                * accessory.bald_preset_sides[2] as i32 as f32,
                                        ),
                                        _ => glam::Vec3::new(
                                            BALD_DEFAULT_INFLUENCE,
                                            BALD_DEFAULT_INFLUENCE,
                                            BALD_DEFAULT_INFLUENCE,
                                        ),
                                    };

                                    let clip_face = match accessory.bald_mode {
                                        BaldMode::Custom => accessory.bald_clip_face as i32,
                                        _ => BALD_DEFAULT_CLIP_FACE as i32,
                                    };

                                    let center = match accessory.bald_mode {
                                        BaldMode::Custom => accessory.bald_center,
                                        _ => BALD_DEFAULT_CENTER,
                                    };

                                    let radius = match accessory.bald_mode {
                                        BaldMode::Custom => accessory.bald_radius,
                                        _ => BALD_DEFAULT_RADIUS,
                                    };

                                    ninja.set_bald(&influence, &center, radius, clip_face);
                                }
                            }
                            self.set_chao_mode(chao_param, &mut ninja);
                            ninja.draw_mdl(device, mdl, chao_global_state.al_body_texlist.clone());
                            ninja.disable_bald();
                        }
                    }
                }
            }

            ninja.set_chao_mode(0, 0);

            self.node_index += 1;
        }

        if let Some(child) = &mut obj.child {
            self.draw_chao(
                chao_param,
                device,
                ninja_state,
                chao_global_state,
                child,
                diff,
                motion,
                accessory,
            );
        }

        ninja_state.borrow_mut().matrix_stack.pop();

        if let Some(sibling) = &mut obj.sibling {
            self.draw_chao(
                chao_param,
                device,
                ninja_state,
                chao_global_state,
                sibling,
                diff,
                motion,
                accessory,
            );
        }
    }
}

pub struct Chao {
    pub chao_param: ChaoParamGc,
    pub chao_global_state: Rc<ChaoGlobalState>,
    pub chao_shape: ChaoShape,
    chao_face: ChaoFace,
    accessory: Option<AccessoryData>,
}

impl Chao {
    pub fn get_type(&self) -> i8 {
        self.chao_param.chao_type
    }

    pub fn get_body_info(&mut self) -> &mut ChaoBodyInfo {
        &mut self.chao_param.body
    }

    pub fn update_type(&mut self, chao_type: i8) {
        self.chao_param.chao_type = chao_type;

        self.chao_shape = ChaoShape::init(&self.chao_param, &self.chao_global_state);
        self.chao_face = ChaoFace::init(&self.chao_param);
    }

    pub fn clear_accessory(&mut self) {
        self.accessory = None;
    }

    pub fn set_accessory(&mut self, accessory_data: &AccessoryData) {
        self.accessory = Some(accessory_data.clone());
    }

    pub fn create(chao_global_state: &Rc<ChaoGlobalState>, chao_param: &ChaoParamGc) -> Self {
        Chao {
            chao_param: *chao_param,
            chao_face: ChaoFace::init(chao_param),
            chao_shape: ChaoShape::init(chao_param, chao_global_state),
            chao_global_state: chao_global_state.clone(),
            accessory: None,
        }
    }

    pub fn update(&mut self) {
        self.chao_shape.deform_shape(&self.chao_param);
        self.chao_shape.deform_palette(&self.chao_param);
    }

    fn before_draw(&mut self, ninja_state: &Rc<RefCell<NinjaState>>) {
        let mut ninja = ninja_state.borrow_mut();
        ninja.set_chao_mode(0, 0);
        ninja.matrix_stack.push();
        ninja.matrix_stack.translate(0.0, -2.0, 0.0);
        ninja.set_colors(&self.chao_shape.palette);
        ninja.set_chao_mode(1, 20);
        ninja.set_chao_alpha_mode(true);
    }

    fn finish_draw(&mut self, ninja_state: &Rc<RefCell<NinjaState>>) {
        let mut ninja = ninja_state.borrow_mut();
        ninja.set_chao_mode(0, 0);
        ninja.matrix_stack.pop();
        ninja.set_chao_alpha_mode(false);
    }

    fn draw_chao(
        &mut self,
        motion: Option<(&NinjaMotion, f32)>,
        device: &wgpu::Device,
        ninja_state: &Rc<RefCell<NinjaState>>,
    ) {
        let prev_rf_mode = ninja_state.borrow().get_renderfix();

        let mut chao_draw = ChaoDraw {
            node_index: 0,
            slope_ang: self.chao_face.get_eyelid_slope_ang(),
            close_ang: self.chao_face.get_eyelid_close_ang(),
            eye_tex_id: self.chao_face.eye_tex_id,
            mouth_tex_id: self.chao_face.mouth_tex_id,
            node_matrices: [Mat4::IDENTITY; 40],
        };
        chao_draw.node_index = 0;

        ninja_state.borrow_mut().set_renderfix(false);
        chao_draw.draw_chao(
            &self.chao_param,
            device,
            ninja_state,
            &self.chao_global_state,
            &mut self.chao_shape.chao_model,
            &self.chao_shape.diff,
            motion,
            &self.accessory,
        );

        if let Some(accessory) = &mut self.accessory {
            ninja_state
                .borrow_mut()
                .set_renderfix(accessory.check_renderfix_render());

            match accessory.accessory_type {
                AccessoryType::Head | AccessoryType::Face => {
                    ninja_state
                        .borrow_mut()
                        .matrix_stack
                        .push_matrix(&chao_draw.node_matrices[16]);
                    ninja_state.borrow_mut().draw(
                        device,
                        &accessory.object,
                        accessory.texlist.clone(),
                    );
                    ninja_state.borrow_mut().matrix_stack.pop();
                }
                AccessoryType::Generic1 | AccessoryType::Generic2 => {
                    for i in 0..40 {
                        let mut counter = 0;
                        let obj = accessory.object.get_node(&mut counter, i);

                        if let Some(obj) = obj {
                            ninja_state
                                .borrow_mut()
                                .matrix_stack
                                .push_matrix(&chao_draw.node_matrices[i]);

                            if let Some(mdl) = &obj.model {
                                ninja_state.borrow_mut().draw_mdl(
                                    device,
                                    mdl,
                                    accessory.texlist.clone(),
                                );
                            }

                            ninja_state.borrow_mut().matrix_stack.pop();
                        }
                    }
                }
            }
        }

        ninja_state.borrow_mut().set_renderfix(prev_rf_mode);
    }

    pub fn render(
        &mut self,
        motion: Option<(&NinjaMotion, f32)>,
        device: &wgpu::Device,
        ninja_state: &Rc<RefCell<NinjaState>>,
    ) {
        self.before_draw(ninja_state);
        self.draw_chao(motion, device, ninja_state);
        self.finish_draw(ninja_state);
    }
}
