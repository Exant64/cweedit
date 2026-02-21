use std::{
    ffi::CStr,
    io::{Cursor, Read, Seek, SeekFrom},
    marker::PhantomData,
    path::PathBuf,
};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use image::{EncodableLayout, ImageReader};

use super::{error::NinjaTexReadError, pak::PAKFile};

pub enum PixelFormat {
    Rgb565,
    Rgb5a3,
    Argb8888,
    Index4,
    Dxt1,
    Dxt5,
    Abgr8888,
}

impl From<u8> for PixelFormat {
    fn from(value: u8) -> Self {
        match value {
            5 => Self::Rgb5a3,
            6 => Self::Argb8888,
            8 => Self::Index4,
            0xE => Self::Dxt1,
            _ => unimplemented!("{}", value),
        }
    }
}

impl From<ddsfile::D3DFormat> for PixelFormat {
    fn from(value: ddsfile::D3DFormat) -> Self {
        match value {
            ddsfile::D3DFormat::R5G6B5 => Self::Rgb565,
            ddsfile::D3DFormat::A8R8G8B8 => Self::Argb8888,
            ddsfile::D3DFormat::A1R5G5B5 => Self::Rgb5a3, // this might be incorrect, but i don't recall any sa2 dds using this anyways
            ddsfile::D3DFormat::DXT1 => Self::Dxt1,
            ddsfile::D3DFormat::DXT5 => Self::Dxt5,
            ddsfile::D3DFormat::A8B8G8R8 => Self::Abgr8888,
            _ => unimplemented!("{:?}", value),
        }
    }
}

impl PixelFormat {
    pub fn get_wgpu_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Rgb565 => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgb5a3 => wgpu::TextureFormat::Rgba8Unorm,
            Self::Argb8888 => wgpu::TextureFormat::Rgba8Unorm,
            Self::Index4 => wgpu::TextureFormat::R8Uint,
            Self::Dxt1 => wgpu::TextureFormat::Bc1RgbaUnorm, // bc1?
            Self::Dxt5 => wgpu::TextureFormat::Bc3RgbaUnorm,
            Self::Abgr8888 => wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    pub fn get_gctex_format(&self) -> gctex::TextureFormat {
        match self {
            Self::Rgb565 => gctex::TextureFormat::RGB565,
            Self::Rgb5a3 => gctex::TextureFormat::RGB5A3,
            Self::Argb8888 => gctex::TextureFormat::RGBA8,
            Self::Index4 => gctex::TextureFormat::I4,
            Self::Dxt1 => gctex::TextureFormat::CMPR,
            _ => unimplemented!(),
        }
    }
}

pub struct NinjaTex {
    pub width: u16,
    pub height: u16,
    pub _flags: u8,
    pub bank: i8,
    pub _ninja_pixel_format: PixelFormat,
    pub real_pixel_format: PixelFormat,
    pub data: Vec<u8>,
}

pub trait NinjaGpuTex<T> {
    fn create_texture(tex_state: &T, tex_entry: &NinjaTex) -> Self;
}

pub struct NinjaTexlist<T: NinjaGpuTex<J>, J> {
    pub textures: Vec<NinjaTex>,
    pub gpu_textures: Vec<T>,

    gpu_tex_state_type: PhantomData<J>,
}

impl NinjaTex {
    pub fn load_gvr(reader: &mut Cursor<&[u8]>) -> Result<(Self, u64), NinjaTexReadError> {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;

        if header != *b"GVRT" {
            return Err(NinjaTexReadError::FormatError("GVR header invalid"));
        }

        let header_size = reader.read_u32::<LittleEndian>()? as u64;

        reader.seek(SeekFrom::Current(2))?;

        // this byte is also used for internal palette format, but we don't support that anyways for now
        let flags = reader.read_u8()? & 0xF;
        let ninja_pixel_format = PixelFormat::from(reader.read_u8()?);

        let width = reader.read_u16::<BigEndian>()?;
        let height = reader.read_u16::<BigEndian>()?;

        let pos = reader.position() as usize;

        // the array data doesn't meet alignment conditions and such on mac for some reason
        // doing this should hopefully meet expectations in most cases
        let texture_size = header_size as usize - 2 * 4;
        let mut texture_data: Vec<u8> = vec![0; texture_size];
        texture_data.clone_from_slice(&reader.get_ref()[pos..(pos + texture_size)]);

        let mut out_pixels = gctex::decode(
            texture_data.as_slice(),
            width as u32,
            height as u32,
            ninja_pixel_format.get_gctex_format(),
            &[],
            0,
        );

        let mut bank = -1;
        let mut real_pixel_format = PixelFormat::Argb8888;
        if let PixelFormat::Index4 = ninja_pixel_format {
            real_pixel_format = PixelFormat::Index4;
            bank = 0;
            let mut skipped_pixels = vec![0u8; width as usize * height as usize];
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let pos = y * width as usize + x;
                    skipped_pixels[pos] = out_pixels[4 * pos];
                }
            }

            out_pixels = skipped_pixels;
        }

        Ok((
            NinjaTex {
                width,
                height,
                _flags: flags,
                bank,
                _ninja_pixel_format: ninja_pixel_format,
                real_pixel_format,
                data: out_pixels,
            },
            header_size,
        ))
    }
}

impl<T, J> NinjaTexlist<T, J>
where
    T: NinjaGpuTex<J>,
{
    pub fn load_pak(
        tex_state: &J,
        pak_name: String,
        buf: &[u8],
    ) -> Result<Self, NinjaTexReadError> {
        const PAK_TEX_INF_SIZE: usize = 60;

        let pak = PAKFile::load_pak(buf)?;
        let inf_name = format!("{}\\{}.inf", pak_name, pak_name);
        let inf_data = pak
            .find_file(&inf_name)
            .ok_or(NinjaTexReadError::FormatError(
                ".inf file couldn't be found!",
            ))?
            .get_data();

        // read inf and find textures
        let tex_count = inf_data.len() / PAK_TEX_INF_SIZE;

        let mut textures: Vec<NinjaTex> = Vec::new();
        let mut gpu_textures = Vec::new();

        let mut reader = Cursor::new(inf_data);
        for _ in 0..tex_count {
            let mut name = vec![0u8; 28];
            // 28 characters for tex name, we don't need it
            reader.read_exact(&mut name)?;
            let name_string = CStr::from_bytes_until_nul(&name)
                .unwrap()
                .to_str()
                .unwrap()
                .to_ascii_lowercase();

            let _global_index = reader.read_u32::<LittleEndian>()?;
            let _type = reader.read_u32::<LittleEndian>()?;
            let _bit_depth = reader.read_u32::<LittleEndian>()?;
            let pixel_format = reader.read_u32::<LittleEndian>()?; // todo: needs & 0xF?
            let width = reader.read_u32::<LittleEndian>()?;
            let height = reader.read_u32::<LittleEndian>()?;
            let _texture_size = reader.read_u32::<LittleEndian>()?;
            let _surface_flags = reader.read_u32::<LittleEndian>()?;

            let file_name = format!("{}\\{}.dds", pak_name, name_string);
            let file_entry = pak
                .find_file(&file_name)
                .ok_or(NinjaTexReadError::FormatError(
                    "couldn't find dds from inf!",
                ))?
                .get_data();

            let tex_entry;
            let dds = ddsfile::Dds::read(Cursor::new(file_entry));
            if let Ok(dds_file) = dds {
                let d3d_format = dds_file.get_d3d_format().unwrap();
                tex_entry = NinjaTex {
                    width: width as u16,
                    height: height as u16,
                    _flags: _surface_flags as u8,
                    bank: -1,
                    _ninja_pixel_format: if pixel_format == 0 {
                        PixelFormat::Argb8888
                    } else {
                        PixelFormat::from(pixel_format as u8)
                    },
                    real_pixel_format: PixelFormat::from(d3d_format),
                    data: dds_file.data,
                };
            } else {
                let img = ImageReader::new(Cursor::new(file_entry))
                    .with_guessed_format()?
                    .decode()
                    .or(Err(NinjaTexReadError::FormatError(
                        "unknown image format in pak!",
                    )))?;
                tex_entry = NinjaTex {
                    width: width as u16,
                    height: height as u16,
                    _flags: _surface_flags as u8,
                    bank: -1,
                    _ninja_pixel_format: PixelFormat::Argb8888,
                    real_pixel_format: PixelFormat::Argb8888,
                    data: img.into_rgba8().as_bytes().to_vec(),
                };
            }

            gpu_textures.push(T::create_texture(tex_state, &tex_entry));
            textures.push(tex_entry);
        }

        Ok(NinjaTexlist {
            textures,
            gpu_textures,
            gpu_tex_state_type: PhantomData,
        })
    }

    pub fn load_gvm(tex_state: &J, buf: &[u8]) -> Result<Self, NinjaTexReadError> {
        let mut textures: Vec<NinjaTex> = Vec::new();
        let mut reader = Cursor::new(buf);

        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;

        if header != *b"GVMH" {
            return Err(NinjaTexReadError::FormatError("GVM header invalid"));
        }

        let header_size = reader.read_u32::<LittleEndian>()? as u64;

        let _flag_thing = reader.read_u16::<BigEndian>()?;
        let tex_count = reader.read_u16::<BigEndian>()?;

        // skipping names for now
        reader.seek(SeekFrom::Start(header_size + 8))?;

        for _i in 0..tex_count {
            let position = reader.position();

            let (texture, size) = NinjaTex::load_gvr(&mut reader)?;
            textures.push(texture);

            reader.seek(SeekFrom::Start(position + size + 8))?;
        }

        let mut gpu_textures = Vec::new();
        for tex_entry in &textures {
            gpu_textures.push(T::create_texture(tex_state, tex_entry));
        }

        Ok(Self {
            textures,
            gpu_textures,
            gpu_tex_state_type: PhantomData,
        })
    }

    pub fn load_tex(tex_state: &J, path: &PathBuf) -> Result<Self, NinjaTexReadError> {
        let data = std::fs::read(path)?;
        if let Some(extension) = &path.extension() {
            if extension.eq_ignore_ascii_case("pak") {
                Self::load_pak(
                    tex_state,
                    path.file_stem()
                        .unwrap()
                        .to_ascii_lowercase()
                        .into_string()
                        .unwrap(),
                    &data,
                )
            } else {
                Self::load_gvm(tex_state, &data)
            }
        } else {
            Err(NinjaTexReadError::IoError())
        }
    }
}
