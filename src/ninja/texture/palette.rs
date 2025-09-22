use std::{
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::PathBuf,
};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use crate::ninja::math::Color;

use super::gvm::PixelFormat;

#[derive(Debug)]
pub enum NinjaPaletteError {
    FormatError(&'static str),
    IoError(),
}

impl From<io::Error> for NinjaPaletteError {
    fn from(_value: io::Error) -> Self {
        Self::IoError()
    }
}

#[derive(Debug)]
pub struct NinjaPalette {
    pub colors: Vec<Color>,
}

impl NinjaPalette {
    // all this code is stolen from gctex crate, I cannot use gctex::decode with height=1
    // because of the way it calculates image size
    const fn convert_3_to_8(v: u8) -> u8 {
        // Swizzle bits: 00000123 -> 12312312
        (v << 5) | (v << 2) | (v >> 1)
    }

    const fn convert_4_to_8(v: u8) -> u8 {
        // Swizzle bits: 00001234 -> 12341234
        (v << 4) | v
    }

    const fn convert_5_to_8(v: u8) -> u8 {
        // Swizzle bits: 00012345 -> 12345123
        (v << 3) | (v >> 2)
    }

    const fn convert_6_to_8(v: u8) -> u8 {
        // Swizzle bits: 00123456 -> 12345612
        (v << 2) | (v >> 4)
    }

    fn decode_pixel_rgb565(val: u16) -> u32 {
        let r = Self::convert_5_to_8(((val >> 11) & 0x1f) as u8);
        let g = Self::convert_6_to_8(((val >> 5) & 0x3f) as u8);
        let b = Self::convert_5_to_8((val & 0x1f) as u8);
        let a = 0xFF;
        (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
    }

    fn decode_pixel_rgb5a3(val: u16) -> u32 {
        let (r, g, b, a) = if (val & 0x8000) != 0 {
            let r = Self::convert_5_to_8(((val >> 10) & 0x1f) as u8);
            let g = Self::convert_5_to_8(((val >> 5) & 0x1f) as u8);
            let b = Self::convert_5_to_8((val & 0x1f) as u8);
            (r, g, b, 0xFF)
        } else {
            let a = Self::convert_3_to_8(((val >> 12) & 0x7) as u8);
            let r = Self::convert_4_to_8(((val >> 8) & 0xf) as u8);
            let g = Self::convert_4_to_8(((val >> 4) & 0xf) as u8);
            let b = Self::convert_4_to_8((val & 0xf) as u8);
            (r, g, b, a)
        };
        (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
    }

    pub fn read_gvp(path: &PathBuf) -> Result<Self, NinjaPaletteError> {
        let data = std::fs::read(path)?;
        Self::load_gvp(&data)
    }

    pub fn load_gvp(buf: &[u8]) -> Result<Self, NinjaPaletteError> {
        let mut reader = Cursor::new(buf);
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;

        if header != *b"GVPL" {
            return Err(NinjaPaletteError::FormatError("GVP header invalid"));
        }

        let _size = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Current(1))?;

        let color_type = match reader.read_u8()? {
            1 => PixelFormat::Rgb565,
            2 => PixelFormat::Rgb5a3,
            _ => unimplemented!(),
        };
        reader.seek(SeekFrom::Current(4))?;

        let mut colors = Vec::new();
        let color_count = reader.read_u16::<BigEndian>()?;
        for _ in 0..color_count {
            let color = match color_type {
                PixelFormat::Rgb565 => Self::decode_pixel_rgb565(reader.read_u16::<BigEndian>()?),
                PixelFormat::Rgb5a3 => Self::decode_pixel_rgb5a3(reader.read_u16::<BigEndian>()?),
                _ => unimplemented!(),
            };
            colors.push(Color::from_slice_rgba(&color.to_le_bytes()));
        }

        Ok(NinjaPalette { colors })
    }
}
