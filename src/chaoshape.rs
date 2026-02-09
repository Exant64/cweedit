use crate::{
    chaoparam::{
        TYPE_AMY, TYPE_CHILD, TYPE_H_FLY, TYPE_H_NORMAL, TYPE_H_POWER, TYPE_H_RUN, TYPE_H_SWIM,
        TYPE_TAILS,
    },
    chaostate::ChaoGlobalState,
    ninja::{
        math::{Color, Point3},
        modelfile::NinjaChunkObject,
        polychunk::PolyChunk,
    },
    ChaoParamGc,
};
use core::panic;
use std::{
    collections::{HashSet, VecDeque},
    rc::Rc,
};

const PART_ATTR_MODEL: u32 = 1;
const PART_ATTR_NODE: u32 = 2;

const PART_ATTR: [u32; 40] = [
    2, 1, 2, 1, 2, 2, 1, 2, 1, 2, 1, 1, 2, 1, 2, 2, 1, 2, 4, 5, 2, 4, 5, 3, 3, 3, 3, 3, 2, 2, 2, 2,
    2, 0, 2, 2, 2, 3, 2, 3,
];

#[repr(u32)]
enum PaletteIndex {
    Ncz = 0x0,
    Hcz = 0x2,
    Hcn = 0x3,
    Hcs = 0x4,
    Hcf = 0x5,
    Hcr = 0x6,
    Hcp = 0x7,
    Dcz = 0x8,
    Hnz = 0x9,
    Hsz = 0xF,
    Hfz = 0x15,
    Hrz = 0x1B,
    Hpz = 0x21,
}

impl From<PaletteIndex> for usize {
    fn from(val: PaletteIndex) -> Self {
        match val {
            PaletteIndex::Ncz => 0x0,
            PaletteIndex::Hcz => 0x2,
            PaletteIndex::Hcn => 0x3,
            PaletteIndex::Hcs => 0x4,
            PaletteIndex::Hcf => 0x5,
            PaletteIndex::Hcr => 0x6,
            PaletteIndex::Hcp => 0x7,
            PaletteIndex::Dcz => 0x8,
            PaletteIndex::Hnz => 0x9,
            PaletteIndex::Hsz => 0xF,
            PaletteIndex::Hfz => 0x15,
            PaletteIndex::Hrz => 0x1B,
            PaletteIndex::Hpz => 0x21,
        }
    }
}

struct ChaoDeformParameters {
    ratio_g: f32,
    ratio_h: f32,
    ratio_v: f32,
    div_ratio_h: f32,
    div_ratio_v: f32,
    alignment: f32,
}

struct ChaoShapeModelSet {
    zero: Vec<NinjaChunkObject>,
    normal: Vec<NinjaChunkObject>,
    swim: Vec<NinjaChunkObject>,
    fly: Vec<NinjaChunkObject>,
    run: Vec<NinjaChunkObject>,
    power: Vec<NinjaChunkObject>,
}

pub struct ChaoShape {
    chao_global_state: Rc<ChaoGlobalState>,
    pub chao_model: NinjaChunkObject,
    pub diff: Vec<Point3>,
    normal_models: ChaoShapeModelSet,
    hero_models: Option<ChaoShapeModelSet>,
    dark_models: Option<ChaoShapeModelSet>,
    pub palette: [Color; 48],
    adjacency_indices: Option<HashSet<u16>>,
    head_hide_parts: bool,
}

impl ChaoShape {
    fn get_object_list(list: &mut Vec<NinjaChunkObject>, object: &NinjaChunkObject) {
        list.push(object.clone());

        if let Some(child) = &object.child {
            Self::get_object_list(list, child);
        }

        if let Some(sibling) = &object.sibling {
            Self::get_object_list(list, sibling);
        }
    }

    fn calc_adjacency_indices(&mut self) -> Option<HashSet<u16>> {
        let mut counter = 0;
        let head = self.chao_model.get_node(&mut counter, 16).unwrap();
        if let Some(model) = &mut head.model {
            let map = model.get_face_adjacency();
            let mut lowest_height = (100000.0, 0);

            for vert in &model.vertex_list {
                for (index, vert) in vert.vertices.iter().enumerate() {
                    if lowest_height.0 > vert.y {
                        lowest_height = (vert.y, index);
                    }
                }
            }

            let lowest_index = lowest_height.1;
            let faces: Vec<(usize, usize, usize)> =
                map.iter().map(|((x, y, z), _)| (*x, *y, *z)).collect();

            let mut components = Vec::new();
            let mut visited = HashSet::new();
            for value in faces {
                if visited.contains(&value) {
                    continue;
                }

                let mut queue = VecDeque::new();
                queue.push_back(value);

                let mut component = HashSet::new();
                while !queue.is_empty() {
                    let f = queue.pop_front().unwrap();
                    if visited.contains(&f) {
                        continue;
                    }
                    visited.insert(f);
                    component.insert((f.0, f.1));
                    queue.extend(map[&f].clone());
                }
                components.push(component);
            }

            let mut final_list = HashSet::new();
            for component in components {
                let mut component_contains_lowest = false;
                let mut vertex_index_list = Vec::new();
                for face in component {
                    if let Some(poly_list) = &model.poly_list {
                        match &poly_list[face.0] {
                            PolyChunk::Strip {
                                flags: _,
                                user_flags: _,
                                strips,
                            } => {
                                let strip = &strips[face.1];
                                if strip.indices.contains(&(lowest_index as u16)) {
                                    component_contains_lowest = true;
                                    break;
                                }
                                vertex_index_list.extend_from_slice(&strip.indices);
                            }
                            _ => continue,
                        }
                    }
                }

                if !component_contains_lowest {
                    final_list.extend(vertex_index_list.as_slice());
                }
            }
            return Some(final_list);
        }
        None
    }

    fn use_adjacency_indices(&mut self) {
        if !self.head_hide_parts {
            return;
        }

        if let Some(indices) = &self.adjacency_indices {
            let mut counter = 0;
            if let Some(head_model) = self
                .chao_model
                .get_node(&mut counter, 16)
                .and_then(|obj| obj.model.as_mut())
            {
                for index in indices {
                    head_model.vertex_list[0].vertices[*index as usize] = Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    };
                }
            }
        }
    }

    pub fn set_hide_head_parts(&mut self, hide: bool) {
        self.head_hide_parts = hide;
    }

    pub fn init(chao_param: &ChaoParamGc, chao_global_state: &Rc<ChaoGlobalState>) -> Self {
        let type_index = 6 * (chao_param.chao_type - 2) as usize;

        let base_model = &chao_global_state.root_objects[type_index];

        let mut child_nodes_vec = Vec::new();
        Self::get_object_list(
            &mut child_nodes_vec,
            &chao_global_state.root_objects[0].clone(),
        );

        let mut zero_nodes_vec = Vec::new();

        Self::get_object_list(
            &mut zero_nodes_vec,
            &chao_global_state.root_objects[type_index].clone(),
        );

        let diff = child_nodes_vec
            .iter()
            .zip(zero_nodes_vec.iter())
            .map(|(child, target)| Point3 {
                x: target.pos.x - child.pos.x,
                y: target.pos.y - child.pos.y,
                z: target.pos.z - child.pos.z,
            })
            .collect();

        let mut shape = ChaoShape {
            chao_global_state: chao_global_state.clone(),
            chao_model: base_model.clone(),
            diff,
            normal_models: ChaoShapeModelSet {
                zero: zero_nodes_vec,
                normal: Vec::new(),
                swim: Vec::new(),
                fly: Vec::new(),
                run: Vec::new(),
                power: Vec::new(),
            },
            hero_models: None,
            dark_models: None,
            palette: [Color::default(); 48],
            adjacency_indices: None,
            head_hide_parts: false,
        };

        Self::get_object_list(
            &mut shape.normal_models.normal,
            &chao_global_state.root_objects[type_index + 1].clone(),
        );
        Self::get_object_list(
            &mut shape.normal_models.swim,
            &chao_global_state.root_objects[type_index + 2].clone(),
        );
        Self::get_object_list(
            &mut shape.normal_models.fly,
            &chao_global_state.root_objects[type_index + 3].clone(),
        );
        Self::get_object_list(
            &mut shape.normal_models.run,
            &chao_global_state.root_objects[type_index + 4].clone(),
        );
        Self::get_object_list(
            &mut shape.normal_models.power,
            &chao_global_state.root_objects[type_index + 5].clone(),
        );

        if chao_param.chao_type == TYPE_CHILD {
            let mut hero_models = ChaoShapeModelSet {
                zero: Vec::new(),
                normal: Vec::new(),
                swim: Vec::new(),
                fly: Vec::new(),
                run: Vec::new(),
                power: Vec::new(),
            };

            let mut dark_models = ChaoShapeModelSet {
                zero: Vec::new(),
                normal: Vec::new(),
                swim: Vec::new(),
                fly: Vec::new(),
                run: Vec::new(),
                power: Vec::new(),
            };

            Self::get_object_list(
                &mut hero_models.zero,
                &chao_global_state.root_objects[6].clone(),
            );
            Self::get_object_list(
                &mut hero_models.normal,
                &chao_global_state.root_objects[7].clone(),
            );
            Self::get_object_list(
                &mut hero_models.swim,
                &chao_global_state.root_objects[8].clone(),
            );
            Self::get_object_list(
                &mut hero_models.fly,
                &chao_global_state.root_objects[9].clone(),
            );
            Self::get_object_list(
                &mut hero_models.run,
                &chao_global_state.root_objects[10].clone(),
            );
            Self::get_object_list(
                &mut hero_models.power,
                &chao_global_state.root_objects[11].clone(),
            );

            Self::get_object_list(
                &mut dark_models.zero,
                &chao_global_state.root_objects[12].clone(),
            );
            Self::get_object_list(
                &mut dark_models.normal,
                &chao_global_state.root_objects[13].clone(),
            );
            Self::get_object_list(
                &mut dark_models.swim,
                &chao_global_state.root_objects[14].clone(),
            );
            Self::get_object_list(
                &mut dark_models.fly,
                &chao_global_state.root_objects[15].clone(),
            );
            Self::get_object_list(
                &mut dark_models.run,
                &chao_global_state.root_objects[16].clone(),
            );
            Self::get_object_list(
                &mut dark_models.power,
                &chao_global_state.root_objects[17].clone(),
            );

            shape.hero_models = Some(hero_models);
            shape.dark_models = Some(dark_models);
        }

        match chao_param.chao_type {
            TYPE_TAILS | TYPE_AMY => {
                shape.adjacency_indices = shape.calc_adjacency_indices();
            }
            _ => {}
        }

        shape.deform_shape(chao_param);
        shape.deform_palette(chao_param);

        shape
    }

    fn get_material_color(poly: &PolyChunk) -> &Color {
        if let PolyChunk::Material {
            source_alpha: _,
            destination_alpha: _,
            diffuse,
            ambient: _,
            specular: _,
        } = poly
        {
            return diffuse.as_ref().unwrap();
        }

        panic!("material color not in right place for chao deform!")
    }

    fn deform_object(
        index: &mut usize,
        object: &mut NinjaChunkObject,
        deform_parameters: &ChaoDeformParameters,
        zero_objects: &Vec<NinjaChunkObject>,
        normal_objects: &Vec<NinjaChunkObject>,
        horizontal_objects: &Vec<NinjaChunkObject>,
        vertical_objects: &Vec<NinjaChunkObject>,
    ) {
        let lerp_closure = |deform_parameters: &ChaoDeformParameters,
                            val_h: f32,
                            val_v: f32,
                            val_n: f32,
                            val_z: f32|
         -> f32 {
            ((val_v * deform_parameters.ratio_v + val_n * (1.0 - deform_parameters.ratio_v))
                * deform_parameters.div_ratio_v
                + (val_h * deform_parameters.ratio_h + val_n * (1.0 - deform_parameters.ratio_h))
                    * deform_parameters.div_ratio_h)
                * deform_parameters.ratio_g
                + val_z * (1.0 - deform_parameters.ratio_g)
        };

        if (PART_ATTR[*index] & PART_ATTR_NODE) != 0 {
            object.pos.x = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].pos.x,
                vertical_objects[*index].pos.x,
                normal_objects[*index].pos.x,
                zero_objects[*index].pos.x,
            );

            object.pos.y = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].pos.y,
                vertical_objects[*index].pos.y,
                normal_objects[*index].pos.y,
                zero_objects[*index].pos.y,
            );

            object.pos.z = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].pos.z,
                vertical_objects[*index].pos.z,
                normal_objects[*index].pos.z,
                zero_objects[*index].pos.z,
            );

            object.ang.x = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].ang.x as f32,
                vertical_objects[*index].ang.x as f32,
                normal_objects[*index].ang.x as f32,
                zero_objects[*index].ang.x as f32,
            ) as i32;

            object.ang.y = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].ang.y as f32,
                vertical_objects[*index].ang.y as f32,
                normal_objects[*index].ang.y as f32,
                zero_objects[*index].ang.y as f32,
            ) as i32;

            object.ang.z = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].ang.z as f32,
                vertical_objects[*index].ang.z as f32,
                normal_objects[*index].ang.z as f32,
                zero_objects[*index].ang.z as f32,
            ) as i32;
        }

        if (PART_ATTR[*index] & PART_ATTR_MODEL) != 0 {
            if let Some(model) = &mut object.model {
                let zero_model = zero_objects[*index].model.as_ref().unwrap();
                let normal_model = normal_objects[*index].model.as_ref().unwrap();
                let horizontal_model = horizontal_objects[*index].model.as_ref().unwrap();
                let vertical_model = vertical_objects[*index].model.as_ref().unwrap();

                if let Some(poly_list) = &mut model.poly_list {
                    for x in 0..poly_list.len() {
                        let poly = &mut poly_list[x];
                        let zero_poly = &zero_model.poly_list.as_ref().unwrap()[x];
                        let normal_poly = &normal_model.poly_list.as_ref().unwrap()[x];
                        let horizontal_poly = &horizontal_model.poly_list.as_ref().unwrap()[x];
                        let vertical_poly = &vertical_model.poly_list.as_ref().unwrap()[x];

                        if let PolyChunk::Material {
                            source_alpha: _,
                            destination_alpha: _,
                            diffuse,
                            ambient: _,
                            specular: _,
                        } = poly
                        {
                            let PolyChunk::Material {
                                source_alpha: _,
                                destination_alpha: _,
                                diffuse: Some(zero_diffuse),
                                ambient: _,
                                specular: _,
                            } = zero_poly
                            else {
                                panic!()
                            };
                            let PolyChunk::Material {
                                source_alpha: _,
                                destination_alpha: _,
                                diffuse: Some(normal_diffuse),
                                ambient: _,
                                specular: _,
                            } = normal_poly
                            else {
                                panic!()
                            };
                            let PolyChunk::Material {
                                source_alpha: _,
                                destination_alpha: _,
                                diffuse: Some(horizontal_diffuse),
                                ambient: _,
                                specular: _,
                            } = horizontal_poly
                            else {
                                panic!()
                            };
                            let PolyChunk::Material {
                                source_alpha: _,
                                destination_alpha: _,
                                diffuse: Some(vertical_diffuse),
                                ambient: _,
                                specular: _,
                            } = vertical_poly
                            else {
                                panic!()
                            };

                            let our_diffuse = diffuse.as_mut().unwrap();

                            our_diffuse.r = lerp_closure(
                                deform_parameters,
                                horizontal_diffuse.r as f32,
                                vertical_diffuse.r as f32,
                                normal_diffuse.r as f32,
                                zero_diffuse.r as f32,
                            ) as u8;

                            our_diffuse.g = lerp_closure(
                                deform_parameters,
                                horizontal_diffuse.g as f32,
                                vertical_diffuse.g as f32,
                                normal_diffuse.g as f32,
                                zero_diffuse.g as f32,
                            ) as u8;

                            our_diffuse.b = lerp_closure(
                                deform_parameters,
                                horizontal_diffuse.b as f32,
                                vertical_diffuse.b as f32,
                                normal_diffuse.b as f32,
                                zero_diffuse.b as f32,
                            ) as u8;

                            break;
                        }
                    }
                }

                let vertex_chunk = &mut model.vertex_list[0];
                for i in 0..vertex_chunk.vertex_count as usize {
                    let vertex_pos = &mut vertex_chunk.vertices[i];

                    let zero_vertex_pos = &zero_model.vertex_list[0].vertices[i];
                    let normal_vertex_pos = &normal_model.vertex_list[0].vertices[i];
                    let horizontal_vertex_pos = &horizontal_model.vertex_list[0].vertices[i];
                    let vertical_vertex_pos = &vertical_model.vertex_list[0].vertices[i];

                    vertex_pos.x = lerp_closure(
                        deform_parameters,
                        horizontal_vertex_pos.x,
                        vertical_vertex_pos.x,
                        normal_vertex_pos.x,
                        zero_vertex_pos.x,
                    );

                    vertex_pos.y = lerp_closure(
                        deform_parameters,
                        horizontal_vertex_pos.y,
                        vertical_vertex_pos.y,
                        normal_vertex_pos.y,
                        zero_vertex_pos.y,
                    );

                    vertex_pos.z = lerp_closure(
                        deform_parameters,
                        horizontal_vertex_pos.z,
                        vertical_vertex_pos.z,
                        normal_vertex_pos.z,
                        zero_vertex_pos.z,
                    );
                }
            }
        }

        if let Some(child) = &mut object.child {
            *index += 1;
            Self::deform_object(
                index,
                child,
                deform_parameters,
                zero_objects,
                normal_objects,
                horizontal_objects,
                vertical_objects,
            );
        }

        if let Some(sibling) = &mut object.sibling {
            *index += 1;
            Self::deform_object(
                index,
                sibling,
                deform_parameters,
                zero_objects,
                normal_objects,
                horizontal_objects,
                vertical_objects,
            );
        }
    }

    fn deform_object_child(
        index: &mut usize,
        object: &mut NinjaChunkObject,
        deform_parameters: &ChaoDeformParameters,
        zero_objects: &Vec<NinjaChunkObject>,
        normal_objects: &Vec<NinjaChunkObject>,
        horizontal_objects: &Vec<NinjaChunkObject>,
        vertical_objects: &Vec<NinjaChunkObject>,
        alignment_zero_objects: &Vec<NinjaChunkObject>,
        alignment_normal_objects: &Vec<NinjaChunkObject>,
        alignment_horizontal_objects: &Vec<NinjaChunkObject>,
        alignment_vertical_objects: &Vec<NinjaChunkObject>,
    ) {
        let interp_angle = |val_a: f32, val_b: f32, t: f32| -> f32 {
            let in_degrees_a = val_a * 360.0 / 65536.0;
            let mut in_degrees_b = val_b * 360.0 / 65536.0 - in_degrees_a;

            while in_degrees_b > 180.0 {
                in_degrees_b -= 360.0;
            }

            (in_degrees_b * t + in_degrees_a) * 65536.0 / 360.0
        };

        let lerp_angle = |deform_parameters: &ChaoDeformParameters,
                          val_h: f32,
                          val_v: f32,
                          val_n: f32,
                          val_z: f32,
                          val_h_a: f32,
                          val_v_a: f32,
                          val_n_a: f32,
                          val_z_a: f32|
         -> f32 {
            let a = interp_angle(
                val_z,
                interp_angle(val_n, val_h, deform_parameters.ratio_h)
                    * deform_parameters.div_ratio_h
                    + interp_angle(val_n, val_v, deform_parameters.ratio_v)
                        * deform_parameters.div_ratio_v,
                deform_parameters.ratio_g,
            );

            let b = interp_angle(
                val_z_a,
                interp_angle(val_n_a, val_h_a, deform_parameters.ratio_h)
                    * deform_parameters.div_ratio_h
                    + interp_angle(val_n_a, val_v_a, deform_parameters.ratio_v)
                        * deform_parameters.div_ratio_v,
                deform_parameters.ratio_g,
            );

            interp_angle(a, b, deform_parameters.alignment)
        };

        let lerp_closure = |deform_parameters: &ChaoDeformParameters,
                            val_h: f32,
                            val_v: f32,
                            val_n: f32,
                            val_z: f32,
                            val_h_a: f32,
                            val_v_a: f32,
                            val_n_a: f32,
                            val_z_a: f32|
         -> f32 {
            let a = ((val_v * deform_parameters.ratio_v
                + val_n * (1.0 - deform_parameters.ratio_v))
                * deform_parameters.div_ratio_v
                + (val_h * deform_parameters.ratio_h + val_n * (1.0 - deform_parameters.ratio_h))
                    * deform_parameters.div_ratio_h)
                * deform_parameters.ratio_g
                + val_z * (1.0 - deform_parameters.ratio_g);

            let b = ((val_v_a * deform_parameters.ratio_v
                + val_n_a * (1.0 - deform_parameters.ratio_v))
                * deform_parameters.div_ratio_v
                + (val_h_a * deform_parameters.ratio_h
                    + val_n_a * (1.0 - deform_parameters.ratio_h))
                    * deform_parameters.div_ratio_h)
                * deform_parameters.ratio_g
                + val_z_a * (1.0 - deform_parameters.ratio_g);

            (b - a) * deform_parameters.alignment + a
        };

        if (PART_ATTR[*index] & PART_ATTR_NODE) != 0 {
            object.pos.x = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].pos.x,
                vertical_objects[*index].pos.x,
                normal_objects[*index].pos.x,
                zero_objects[*index].pos.x,
                alignment_horizontal_objects[*index].pos.x,
                alignment_vertical_objects[*index].pos.x,
                alignment_normal_objects[*index].pos.x,
                alignment_zero_objects[*index].pos.x,
            );

            object.pos.y = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].pos.y,
                vertical_objects[*index].pos.y,
                normal_objects[*index].pos.y,
                zero_objects[*index].pos.y,
                alignment_horizontal_objects[*index].pos.y,
                alignment_vertical_objects[*index].pos.y,
                alignment_normal_objects[*index].pos.y,
                alignment_zero_objects[*index].pos.y,
            );

            object.pos.z = lerp_closure(
                deform_parameters,
                horizontal_objects[*index].pos.z,
                vertical_objects[*index].pos.z,
                normal_objects[*index].pos.z,
                zero_objects[*index].pos.z,
                alignment_horizontal_objects[*index].pos.z,
                alignment_vertical_objects[*index].pos.z,
                alignment_normal_objects[*index].pos.z,
                alignment_zero_objects[*index].pos.z,
            );

            object.ang.x = lerp_angle(
                deform_parameters,
                horizontal_objects[*index].ang.x as f32,
                vertical_objects[*index].ang.x as f32,
                normal_objects[*index].ang.x as f32,
                zero_objects[*index].ang.x as f32,
                alignment_horizontal_objects[*index].ang.x as f32,
                alignment_vertical_objects[*index].ang.x as f32,
                alignment_normal_objects[*index].ang.x as f32,
                alignment_zero_objects[*index].ang.x as f32,
            ) as i32;

            object.ang.y = lerp_angle(
                deform_parameters,
                horizontal_objects[*index].ang.y as f32,
                vertical_objects[*index].ang.y as f32,
                normal_objects[*index].ang.y as f32,
                zero_objects[*index].ang.y as f32,
                alignment_horizontal_objects[*index].ang.y as f32,
                alignment_vertical_objects[*index].ang.y as f32,
                alignment_normal_objects[*index].ang.y as f32,
                alignment_zero_objects[*index].ang.y as f32,
            ) as i32;

            object.ang.z = lerp_angle(
                deform_parameters,
                horizontal_objects[*index].ang.z as f32,
                vertical_objects[*index].ang.z as f32,
                normal_objects[*index].ang.z as f32,
                zero_objects[*index].ang.z as f32,
                alignment_horizontal_objects[*index].ang.z as f32,
                alignment_vertical_objects[*index].ang.z as f32,
                alignment_normal_objects[*index].ang.z as f32,
                alignment_zero_objects[*index].ang.z as f32,
            ) as i32;
        }

        if (PART_ATTR[*index] & PART_ATTR_MODEL) != 0 {
            if let Some(model) = &mut object.model {
                let zero_model = zero_objects[*index].model.as_ref().unwrap();
                let normal_model = normal_objects[*index].model.as_ref().unwrap();
                let horizontal_model = horizontal_objects[*index].model.as_ref().unwrap();
                let vertical_model = vertical_objects[*index].model.as_ref().unwrap();
                let alignment_zero_model = alignment_zero_objects[*index].model.as_ref().unwrap();
                let alignment_normal_model =
                    alignment_normal_objects[*index].model.as_ref().unwrap();
                let alignment_horizontal_model =
                    alignment_horizontal_objects[*index].model.as_ref().unwrap();
                let alignment_vertical_model =
                    alignment_vertical_objects[*index].model.as_ref().unwrap();

                if let Some(poly_list) = &mut model.poly_list {
                    for x in 0..poly_list.len() {
                        let poly = &mut poly_list[x];
                        let zero_poly = &zero_model.poly_list.as_ref().unwrap()[x];
                        let normal_poly = &normal_model.poly_list.as_ref().unwrap()[x];
                        let horizontal_poly = &horizontal_model.poly_list.as_ref().unwrap()[x];
                        let vertical_poly = &vertical_model.poly_list.as_ref().unwrap()[x];
                        let alignment_zero_poly =
                            &alignment_zero_model.poly_list.as_ref().unwrap()[x];
                        let alignment_normal_poly =
                            &alignment_normal_model.poly_list.as_ref().unwrap()[x];
                        let alignment_horizontal_poly =
                            &alignment_horizontal_model.poly_list.as_ref().unwrap()[x];
                        let alignment_vertical_poly =
                            &alignment_vertical_model.poly_list.as_ref().unwrap()[x];

                        if let PolyChunk::Material {
                            source_alpha: _,
                            destination_alpha: _,
                            diffuse,
                            ambient: _,
                            specular: _,
                        } = poly
                        {
                            let zero_diffuse = Self::get_material_color(zero_poly);
                            let normal_diffuse = Self::get_material_color(normal_poly);
                            let horizontal_diffuse = Self::get_material_color(horizontal_poly);
                            let vertical_diffuse = Self::get_material_color(vertical_poly);

                            let alignment_zero_diffuse =
                                Self::get_material_color(alignment_zero_poly);
                            let alignment_normal_diffuse =
                                Self::get_material_color(alignment_normal_poly);
                            let alignment_horizontal_diffuse =
                                Self::get_material_color(alignment_horizontal_poly);
                            let alignment_vertical_diffuse =
                                Self::get_material_color(alignment_vertical_poly);

                            let our_diffuse = diffuse.as_mut().unwrap();

                            our_diffuse.r = lerp_closure(
                                deform_parameters,
                                horizontal_diffuse.r as f32,
                                vertical_diffuse.r as f32,
                                normal_diffuse.r as f32,
                                zero_diffuse.r as f32,
                                alignment_horizontal_diffuse.r as f32,
                                alignment_vertical_diffuse.r as f32,
                                alignment_normal_diffuse.r as f32,
                                alignment_zero_diffuse.r as f32,
                            ) as u8;

                            our_diffuse.g = lerp_closure(
                                deform_parameters,
                                horizontal_diffuse.g as f32,
                                vertical_diffuse.g as f32,
                                normal_diffuse.g as f32,
                                zero_diffuse.g as f32,
                                alignment_horizontal_diffuse.g as f32,
                                alignment_vertical_diffuse.g as f32,
                                alignment_normal_diffuse.g as f32,
                                alignment_zero_diffuse.g as f32,
                            ) as u8;

                            our_diffuse.b = lerp_closure(
                                deform_parameters,
                                horizontal_diffuse.b as f32,
                                vertical_diffuse.b as f32,
                                normal_diffuse.b as f32,
                                zero_diffuse.b as f32,
                                alignment_horizontal_diffuse.b as f32,
                                alignment_vertical_diffuse.b as f32,
                                alignment_normal_diffuse.b as f32,
                                alignment_zero_diffuse.b as f32,
                            ) as u8;

                            break;
                        }
                    }
                }

                let vertex_chunk = &mut model.vertex_list[0];
                for i in 0..vertex_chunk.vertex_count as usize {
                    let vertex_pos = &mut vertex_chunk.vertices[i];

                    let zero_vertex_pos = &zero_model.vertex_list[0].vertices[i];
                    let normal_vertex_pos = &normal_model.vertex_list[0].vertices[i];
                    let horizontal_vertex_pos = &horizontal_model.vertex_list[0].vertices[i];
                    let vertical_vertex_pos = &vertical_model.vertex_list[0].vertices[i];

                    let alignment_zero_vertex_pos =
                        &alignment_zero_model.vertex_list[0].vertices[i];
                    let alignment_normal_vertex_pos =
                        &alignment_normal_model.vertex_list[0].vertices[i];
                    let alignment_horizontal_vertex_pos =
                        &alignment_horizontal_model.vertex_list[0].vertices[i];
                    let alignment_vertical_vertex_pos =
                        &alignment_vertical_model.vertex_list[0].vertices[i];

                    vertex_pos.x = lerp_closure(
                        deform_parameters,
                        horizontal_vertex_pos.x,
                        vertical_vertex_pos.x,
                        normal_vertex_pos.x,
                        zero_vertex_pos.x,
                        alignment_horizontal_vertex_pos.x,
                        alignment_vertical_vertex_pos.x,
                        alignment_normal_vertex_pos.x,
                        alignment_zero_vertex_pos.x,
                    );

                    vertex_pos.y = lerp_closure(
                        deform_parameters,
                        horizontal_vertex_pos.y,
                        vertical_vertex_pos.y,
                        normal_vertex_pos.y,
                        zero_vertex_pos.y,
                        alignment_horizontal_vertex_pos.y,
                        alignment_vertical_vertex_pos.y,
                        alignment_normal_vertex_pos.y,
                        alignment_zero_vertex_pos.y,
                    );

                    vertex_pos.z = lerp_closure(
                        deform_parameters,
                        horizontal_vertex_pos.z,
                        vertical_vertex_pos.z,
                        normal_vertex_pos.z,
                        zero_vertex_pos.z,
                        alignment_horizontal_vertex_pos.z,
                        alignment_vertical_vertex_pos.z,
                        alignment_normal_vertex_pos.z,
                        alignment_zero_vertex_pos.z,
                    );
                }
            }
        }

        if let Some(child) = &mut object.child {
            *index += 1;
            Self::deform_object_child(
                index,
                child,
                deform_parameters,
                zero_objects,
                normal_objects,
                horizontal_objects,
                vertical_objects,
                alignment_zero_objects,
                alignment_normal_objects,
                alignment_horizontal_objects,
                alignment_vertical_objects,
            );
        }

        if let Some(sibling) = &mut object.sibling {
            *index += 1;
            Self::deform_object_child(
                index,
                sibling,
                deform_parameters,
                zero_objects,
                normal_objects,
                horizontal_objects,
                vertical_objects,
                alignment_zero_objects,
                alignment_normal_objects,
                alignment_horizontal_objects,
                alignment_vertical_objects,
            );
        }
    }

    pub fn deform_shape(&mut self, chao_param: &ChaoParamGc) {
        let ratio_g = chao_param.body.growth;
        let mut ratio_h = chao_param.body.h_pos;
        let mut ratio_v = chao_param.body.v_pos;

        let zero_objects = &self.normal_models.zero;
        let normal_objects = &self.normal_models.normal;
        let horiz_objects = if ratio_h < 0.0 {
            &self.normal_models.swim
        } else {
            &self.normal_models.fly
        };

        let vert_objects = if ratio_v < 0.0 {
            &self.normal_models.run
        } else {
            &self.normal_models.power
        };

        if ratio_h == 0.0 {
            ratio_h = 0.000001;
        } else if ratio_h < 0.0 {
            ratio_h = -ratio_h;
        }

        if ratio_v == 0.0 {
            ratio_v = 0.000001;
        } else if ratio_v < 0.0 {
            ratio_v = -ratio_v;
        }

        let div_ratio = 1.0 / (ratio_h + ratio_v);
        let div_ratio_h = ratio_h * div_ratio;
        let div_ratio_v = ratio_v * div_ratio;

        let deform_params = ChaoDeformParameters {
            ratio_g,
            ratio_h,
            ratio_v,
            div_ratio_h,
            div_ratio_v,
            alignment: chao_param.body.a_pos.abs(),
        };

        let mut index = 0usize;
        if chao_param.chao_type == TYPE_CHILD {
            let alignment_models = if chao_param.body.a_pos >= 0.0 {
                &self.hero_models.as_ref().unwrap()
            } else {
                &self.dark_models.as_ref().unwrap()
            };

            let alignment_zero_objects = &alignment_models.zero;
            let alignment_normal_objects = &alignment_models.normal;
            let alignment_horiz_objects = if chao_param.body.h_pos < 0.0 {
                &alignment_models.swim
            } else {
                &alignment_models.fly
            };

            let alignment_vert_objects = if chao_param.body.v_pos < 0.0 {
                &alignment_models.run
            } else {
                &alignment_models.power
            };

            Self::deform_object_child(
                &mut index,
                &mut self.chao_model,
                &deform_params,
                zero_objects,
                normal_objects,
                horiz_objects,
                vert_objects,
                alignment_zero_objects,
                alignment_normal_objects,
                alignment_horiz_objects,
                alignment_vert_objects,
            );
        } else {
            Self::deform_object(
                &mut index,
                &mut self.chao_model,
                &deform_params,
                zero_objects,
                normal_objects,
                horiz_objects,
                vert_objects,
            );
        }

        self.use_adjacency_indices();
    }

    pub fn deform_palette(&mut self, chao_param: &ChaoParamGc) {
        let palette_base_index: usize = match chao_param.chao_type {
            TYPE_CHILD => PaletteIndex::Ncz,
            TYPE_H_NORMAL => PaletteIndex::Hnz,
            TYPE_H_SWIM => PaletteIndex::Hsz,
            TYPE_H_FLY => PaletteIndex::Hfz,
            TYPE_H_RUN => PaletteIndex::Hrz,
            TYPE_H_POWER => PaletteIndex::Hpz,
            _ => return,
        }
        .into();

        let ratio_g = chao_param.body.growth;
        let mut ratio_h = chao_param.body.h_pos;
        let mut ratio_v = chao_param.body.v_pos;

        let palette_zero = &self.chao_global_state.palettes[palette_base_index];
        let palette_normal = &self.chao_global_state.palettes[palette_base_index + 1];

        let palette_horizontal = if ratio_h < 0.0 {
            &self.chao_global_state.palettes[palette_base_index + 4]
        } else {
            &self.chao_global_state.palettes[palette_base_index + 5]
        };

        let palette_vertical = if ratio_v < 0.0 {
            &self.chao_global_state.palettes[palette_base_index + 2]
        } else {
            &self.chao_global_state.palettes[palette_base_index + 3]
        };

        if ratio_h == 0.0 {
            ratio_h = 0.000001;
        } else if ratio_h < 0.0 {
            ratio_h = -ratio_h;
        }

        if ratio_v == 0.0 {
            ratio_v = 0.000001;
        } else if ratio_v < 0.0 {
            ratio_v = -ratio_v;
        }

        let div_ratio = 1.0 / (ratio_h + ratio_v);
        let div_ratio_h = ratio_h * div_ratio;
        let div_ratio_v = ratio_v * div_ratio;

        let lerp = |val_a: u8, val_b: u8, ratio: f32| -> u8 {
            ((1.0 - ratio) * val_a as f32 + val_b as f32 * ratio) as u8
        };

        let lerp_closure = move |color_h: u8, color_v: u8, color_n: u8, color_z: u8| -> u8 {
            (((color_v as f32 * ratio_v + color_n as f32 * (1.0 - ratio_v)) * div_ratio_v
                + (color_h as f32 * ratio_h + color_n as f32 * (1.0 - ratio_h)) * div_ratio_h)
                * ratio_g
                + color_z as f32 * (1.0 - ratio_g)) as u8
        };

        for i in 0..self.palette.len() {
            if chao_param.chao_type == TYPE_CHILD {
                let n_r = lerp(
                    palette_zero.colors[i].r,
                    palette_normal.colors[i].r,
                    ratio_g,
                );

                let n_g = lerp(
                    palette_zero.colors[i].g,
                    palette_normal.colors[i].g,
                    ratio_g,
                );

                let n_b = lerp(
                    palette_zero.colors[i].b,
                    palette_normal.colors[i].b,
                    ratio_g,
                );

                let hero_palette_zero =
                    &self.chao_global_state.palettes[PaletteIndex::Hcz as usize];
                let hero_palette_normal =
                    &self.chao_global_state.palettes[PaletteIndex::Hcn as usize];

                let hero_palette_horizontal = if chao_param.body.h_pos < 0.0 {
                    &self.chao_global_state.palettes[PaletteIndex::Hcs as usize]
                } else {
                    &self.chao_global_state.palettes[PaletteIndex::Hcf as usize]
                };

                let hero_palette_vertical = if chao_param.body.v_pos < 0.0 {
                    &self.chao_global_state.palettes[PaletteIndex::Hcr as usize]
                } else {
                    &self.chao_global_state.palettes[PaletteIndex::Hcp as usize]
                };

                let dark_palette_child =
                    &self.chao_global_state.palettes[PaletteIndex::Dcz as usize];

                let ratio_a = chao_param.body.a_pos.abs().min(1.0);
                if chao_param.body.a_pos >= 0.0 {
                    let a_r = lerp_closure(
                        hero_palette_horizontal.colors[i].r,
                        hero_palette_vertical.colors[i].r,
                        hero_palette_normal.colors[i].r,
                        hero_palette_zero.colors[i].r,
                    );

                    let a_g = lerp_closure(
                        hero_palette_horizontal.colors[i].g,
                        hero_palette_vertical.colors[i].g,
                        hero_palette_normal.colors[i].g,
                        hero_palette_zero.colors[i].g,
                    );

                    let a_b = lerp_closure(
                        hero_palette_horizontal.colors[i].b,
                        hero_palette_vertical.colors[i].b,
                        hero_palette_normal.colors[i].b,
                        hero_palette_zero.colors[i].b,
                    );

                    self.palette[i].r = lerp(n_r, a_r, ratio_a);
                    self.palette[i].g = lerp(n_g, a_g, ratio_a);
                    self.palette[i].b = lerp(n_b, a_b, ratio_a);
                } else {
                    self.palette[i].r = lerp(n_r, dark_palette_child.colors[i].r, ratio_a);
                    self.palette[i].g = lerp(n_g, dark_palette_child.colors[i].g, ratio_a);
                    self.palette[i].b = lerp(n_b, dark_palette_child.colors[i].b, ratio_a);
                }

                continue;
            }

            self.palette[i].r = lerp_closure(
                palette_horizontal.colors[i].r,
                palette_vertical.colors[i].r,
                palette_normal.colors[i].r,
                palette_zero.colors[i].r,
            );

            self.palette[i].g = lerp_closure(
                palette_horizontal.colors[i].g,
                palette_vertical.colors[i].g,
                palette_normal.colors[i].g,
                palette_zero.colors[i].g,
            );

            self.palette[i].b = lerp_closure(
                palette_horizontal.colors[i].b,
                palette_vertical.colors[i].b,
                palette_normal.colors[i].b,
                palette_zero.colors[i].b,
            );
        }
    }
}
