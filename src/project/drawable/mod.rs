use std::{cell::RefCell, rc::Rc};

use crate::ninja::ninjadraw::NinjaState;

pub mod chaodraw;

pub trait Drawable {
    fn draw(&mut self, device: &wgpu::Device, ninja: &Rc<RefCell<NinjaState>>);
}
