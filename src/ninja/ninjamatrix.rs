use std::{collections::VecDeque, f32::consts};

use super::math::NinjaRotation;

pub struct NinjaMatrixStack {
    stack: VecDeque<glam::Mat4>,
}

impl NinjaMatrixStack {
    pub fn init() -> Self {
        let mut stack = VecDeque::new();
        stack.push_back(glam::Mat4::IDENTITY);

        NinjaMatrixStack { stack }
    }

    pub fn push_matrix(&mut self, matrix: &glam::Mat4) {
        self.stack.push_front(*matrix);
    }

    pub fn push(&mut self) {
        let top = self.stack[0];
        self.push_matrix(&top);
    }

    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.stack[0] *= glam::Mat4::from_translation(glam::Vec3 { x, y, z });
    }

    pub fn rotate(&mut self, ang: &NinjaRotation) {
        const ANG_TO_RAD: f32 = consts::TAU / 65536.0;
        self.stack[0] *= glam::Mat4::from_rotation_z(ang.z as f32 * ANG_TO_RAD);
        self.stack[0] *= glam::Mat4::from_rotation_y(ang.y as f32 * ANG_TO_RAD);
        self.stack[0] *= glam::Mat4::from_rotation_x(ang.x as f32 * ANG_TO_RAD);
    }

    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.stack[0] *= glam::Mat4::from_scale(glam::Vec3 { x, y, z });
    }

    pub fn pop(&mut self) {
        self.stack.pop_front();
    }

    pub fn get(&self) -> glam::Mat4 {
        return self.stack[0];
    }
}
