use byteorder::{LittleEndian, ReadBytesExt};
use std::{io::Read, u8};

use super::error::NinjaParseError;

#[derive(Debug, Clone, Copy)]
pub struct Float2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct NinjaRotation {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
}

impl Float2 {
    pub fn from_buf_u16<R: Read>(reader: &mut R, half: bool) -> Result<Self, NinjaParseError> {
        let mul = if half { 1.0 / 256.0 } else { 1.0 / 1024.0 };

        Ok(Float2 {
            x: mul * reader.read_i16::<LittleEndian>()? as f32,
            y: mul * reader.read_i16::<LittleEndian>()? as f32,
        })
    }
}

impl Point3 {
    pub fn lerp(&self, b: &Point3, t: f32) -> Point3 {
        Point3 {
            x: self.x + (b.x - self.x) * t,
            y: self.y + (b.y - self.y) * t,
            z: self.z + (b.z - self.z) * t,
        }
    }

    pub fn from_buf<R: Read>(reader: &mut R) -> Result<Self, NinjaParseError> {
        Ok(Point3 {
            x: reader.read_f32::<LittleEndian>()?,
            y: reader.read_f32::<LittleEndian>()?,
            z: reader.read_f32::<LittleEndian>()?,
        })
    }
}

impl NinjaRotation {
    pub fn lerp(&self, b: &NinjaRotation, t: f32) -> NinjaRotation {
        NinjaRotation {
            x: (self.x as f32 + (b.x - self.x) as f32 * t) as i32,
            y: (self.y as f32 + (b.y - self.y) as f32 * t) as i32,
            z: (self.z as f32 + (b.z - self.z) as f32 * t) as i32,
        }
    }
    pub fn from_buf<R: Read>(reader: &mut R) -> Result<Self, NinjaParseError> {
        Ok(NinjaRotation {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
            z: reader.read_i32::<LittleEndian>()?,
        })
    }

    pub fn from_buf_16<R: Read>(reader: &mut R) -> Result<Self, NinjaParseError> {
        Ok(NinjaRotation {
            x: reader.read_i16::<LittleEndian>()? as i32,
            y: reader.read_i16::<LittleEndian>()? as i32,
            z: reader.read_i16::<LittleEndian>()? as i32,
        })
    }
}

impl Color {
    pub fn from_buf_16<R: Read>(reader: &mut R) -> Self {
        Color {
            b: reader.read_u8().unwrap(),
            g: reader.read_u8().unwrap(),
            r: reader.read_u8().unwrap(),
            a: reader.read_u8().unwrap(),
        }
    }

    pub fn from_slice_rgba(slice: &[u8]) -> Self {
        Color {
            r: slice[0],
            g: slice[1],
            b: slice[2],
            a: slice[3],
        }
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Color {
            b: slice[0],
            g: slice[1],
            r: slice[2],
            a: slice[3],
        }
    }

    pub fn from_buf<R: Read>(reader: &mut R) -> Result<Self, NinjaParseError> {
        Ok(Color {
            b: reader.read_u8()?,
            g: reader.read_u8()?,
            r: reader.read_u8()?,
            a: reader.read_u8()?,
        })
    }
}
