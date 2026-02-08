use super::{
    error::NinjaParseError,
    math::{Color, Float2},
    AlphaInstruction, FilterMode,
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    io::{Read, Seek, SeekFrom}
};

pub enum PolyChunkType {
    Strip,
    StripUVN,
    StripUVH,
    StripNormal,
    StripUVNNormal,
    StripUVHNormal,
    StripColor,
    StripUVNColor,
    StripUVHColor,
    Strip2,
    StripUVN2,
    StripUVH2,
    End,
}

impl TryFrom<u8> for PolyChunkType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            64 => Ok(PolyChunkType::Strip),
            65 => Ok(PolyChunkType::StripUVN),
            66 => Ok(PolyChunkType::StripUVH),
            67 => Ok(PolyChunkType::StripNormal),
            68 => Ok(PolyChunkType::StripUVNNormal),
            69 => Ok(PolyChunkType::StripUVHNormal),
            70 => Ok(PolyChunkType::StripColor),
            71 => Ok(PolyChunkType::StripUVNColor),
            72 => Ok(PolyChunkType::StripUVHColor),
            73 => Ok(PolyChunkType::Strip2),
            74 => Ok(PolyChunkType::StripUVN2),
            75 => Ok(PolyChunkType::StripUVH2),
            255 => Ok(PolyChunkType::End),
            _ => Err(()),
        }
    }
}

impl PolyChunkType {
    pub fn has_uv(&self) -> bool {
        matches!(
            self,
            PolyChunkType::StripUVN
                | PolyChunkType::StripUVH
                | PolyChunkType::StripUVNNormal
                | PolyChunkType::StripUVHNormal
                | PolyChunkType::StripUVNColor
                | PolyChunkType::StripUVHColor
                | PolyChunkType::StripUVN2
                | PolyChunkType::StripUVH2
        )
    }

    pub fn has_uv2(&self) -> bool {
        matches!(self, PolyChunkType::StripUVN2 | PolyChunkType::StripUVH2)
    }

    pub fn has_normals(&self) -> bool {
        matches!(
            self,
            PolyChunkType::StripNormal
                | PolyChunkType::StripUVNNormal
                | PolyChunkType::StripUVHNormal
        )
    }

    pub fn has_colors(&self) -> bool {
        matches!(
            self,
            PolyChunkType::StripColor | PolyChunkType::StripUVNColor | PolyChunkType::StripUVHColor
        )
    }
}

#[derive(Debug, Clone)]
pub struct Strip {
    pub reversed: bool,
    pub indices: Box<[u16]>,
    pub uvs: Option<Box<[Float2]>>,
    pub uvs2: Option<Box<[Float2]>>,
    pub colors: Option<Box<[Color]>>,
    pub user_flags_1: Option<Box<[u16]>>,
    pub user_flags_2: Option<Box<[u16]>>,
    pub user_flags_3: Option<Box<[u16]>>,
}

impl Strip {
    pub fn from_buf<R: Read>(
        reader: &mut R,
        chunk_type: PolyChunkType,
        user_flags_count: u32,
    ) -> Result<Self, NinjaParseError> {
        // TODO: no way that this has to be this ugly lol
        let read_count = reader.read_i16::<LittleEndian>()?;
        let count = read_count.unsigned_abs() as usize;

        let has_uv = chunk_type.has_uv();
        let has_uv2 = chunk_type.has_uv2();
        let has_vcolors = chunk_type.has_colors();

        let uv_half = !matches!(
            chunk_type,
            PolyChunkType::StripUVH
                | PolyChunkType::StripUVHNormal
                | PolyChunkType::StripUVHColor
                | PolyChunkType::StripUVH2
        );

        let mut indices = vec![0u16; count].into_boxed_slice();

        let mut uvs = if has_uv {
            Some(Vec::<Float2>::with_capacity(count))
        } else {
            None
        };
        let mut uvs2 = if has_uv2 {
            Some(Vec::<Float2>::with_capacity(count))
        } else {
            None
        };
        let mut colors = if has_vcolors {
            Some(Vec::<Color>::with_capacity(count))
        } else {
            None
        };
        let mut user_flags_1 = if user_flags_count >= 1 {
            Some(Vec::<u16>::with_capacity(count - 2))
        } else {
            None
        };
        let mut user_flags_2 = if user_flags_count >= 2 {
            Some(Vec::<u16>::with_capacity(count - 2))
        } else {
            None
        };
        let mut user_flags_3 = if user_flags_count >= 3 {
            Some(Vec::<u16>::with_capacity(count - 2))
        } else {
            None
        };

        for i in 0..count {
            indices[i] = reader.read_u16::<LittleEndian>()?;

            if has_uv {
                uvs.as_mut()
                    .unwrap()
                    .push(Float2::from_buf_u16(reader, uv_half)?);
            }

            if has_uv2 {
                uvs2.as_mut()
                    .unwrap()
                    .push(Float2::from_buf_u16(reader, uv_half)?);
            }

            if has_vcolors {
                colors.as_mut().unwrap().push(Color::from_buf(reader)?);
            }

            if i >= 2 {
                if user_flags_count >= 1 {
                    user_flags_1
                        .as_mut()
                        .unwrap()
                        .push(reader.read_u16::<LittleEndian>()?);
                }

                if user_flags_count >= 2 {
                    user_flags_2
                        .as_mut()
                        .unwrap()
                        .push(reader.read_u16::<LittleEndian>()?);
                }

                if user_flags_count >= 3 {
                    user_flags_3
                        .as_mut()
                        .unwrap()
                        .push(reader.read_u16::<LittleEndian>()?);
                }
            }
        }

        Ok(Strip {
            reversed: read_count < 0,
            indices,
            uvs: uvs.map(|x| x.into_boxed_slice()),
            uvs2: uvs2.map(|x| x.into_boxed_slice()),
            colors: colors.map(|x| x.into_boxed_slice()),
            user_flags_1: user_flags_1.map(|x| x.into_boxed_slice()),
            user_flags_2: user_flags_2.map(|x| x.into_boxed_slice()),
            user_flags_3: user_flags_3.map(|x| x.into_boxed_slice()),
        })
    }
}

#[derive(Debug, Clone)]
pub enum PolyChunk {
    Null,
    BitsBlendAlpha {
        source_alpha: AlphaInstruction,
        destination_alpha: AlphaInstruction,
    },
    BitsMipmapDAdjust(f32),
    BitsSpecularExponent(u8),
    BitsCachePolygonList(u8),
    BitsDrawPolygonList(u8),
    TinyTextureID {
        mipmap_d_adjust: f32,
        clamp_u: bool,
        clamp_v: bool,
        flip_u: bool,
        flip_v: bool,

        texture_id: u16,
        super_sample: bool,
        filter_mode: FilterMode,
    },
    Material {
        source_alpha: AlphaInstruction,
        destination_alpha: AlphaInstruction,
        diffuse: Option<Color>,
        ambient: Option<Color>,
        specular: Option<Color>,
    },
    MaterialBump {
        dx: u16,
        dy: u16,
        dz: u16,

        ux: u16,
        uy: u16,
        uz: u16,
    },
    Strip {
        flags: u8,
        user_flags: u8,
        strips: Vec<Strip>,
    },
}

impl PolyChunk {
    pub fn from_buf<R: Read + Seek>(
        reader: &mut R,
    ) -> Result<Option<(Vec<u8>, Self)>, NinjaParseError> {
        let plist_start = reader.stream_position().unwrap();
        let plist_header = reader.read_u16::<LittleEndian>()?;
        let plist_type = (plist_header & 0xFF) as u32;
        let plist_flags = (plist_header >> 8) as u8;

        const NJD_NULLOFF: u32 = 0;
        const NJD_BITSOFF: u32 = 1;
        const NJD_CB_BA: u32 = NJD_BITSOFF;
        const NJD_CB_DA: u32 = NJD_BITSOFF + 1;
        const NJD_CB_EXP: u32 = NJD_BITSOFF + 2;
        const NJD_CB_CP: u32 = NJD_BITSOFF + 3;
        const NJD_CB_DP: u32 = NJD_BITSOFF + 4;
        const NJD_TINYOFF: u32 = 8;
        const NJD_MATOFF: u32 = 16;
        const NJD_VERTOFF: u32 = 32;
        const NJD_STRIPOFF: u32 = 64;
        const NJD_STRIPOFF_END: u32 = 64 + 12;
        const NJD_ENDOFF: u32 = 255;

        let plist: Result<Option<PolyChunk>, NinjaParseError> = match plist_type {
            NJD_NULLOFF => Ok(Some(PolyChunk::Null)),

            NJD_CB_BA => Ok(Some(PolyChunk::BitsBlendAlpha {
                source_alpha: AlphaInstruction::try_from((plist_flags >> 3) & 7).unwrap(),
                destination_alpha: AlphaInstruction::try_from(plist_flags & 7).unwrap(),
            })),
            NJD_CB_DA => Ok(Some(PolyChunk::BitsMipmapDAdjust(
                0.25 * (plist_flags & 0xF) as f32,
            ))),
            NJD_CB_EXP => Ok(Some(PolyChunk::BitsSpecularExponent(plist_flags & 0x1F))),
            NJD_CB_CP => Ok(Some(PolyChunk::BitsCachePolygonList(plist_flags))),
            NJD_CB_DP => Ok(Some(PolyChunk::BitsDrawPolygonList(plist_flags))),

            NJD_TINYOFF..NJD_MATOFF => {
                let data = reader.read_u16::<LittleEndian>()?;

                Ok(Some(PolyChunk::TinyTextureID {
                    mipmap_d_adjust: 0.25 * (plist_flags & 0xF) as f32,
                    clamp_u: (plist_flags & 0x20) == 0x20,
                    clamp_v: (plist_flags & 0x10) == 0x10,
                    flip_u: (plist_flags & 0x80) == 0x80,
                    flip_v: (plist_flags & 0x40) == 0x40,
                    texture_id: (data & 0x1FFF),
                    super_sample: (data & 0x2000) == 0x2000,
                    filter_mode: FilterMode::try_from((data >> 14) as u8).unwrap(),
                }))
            }

            NJD_MATOFF..NJD_VERTOFF => {
                let mat_type = plist_type - NJD_MATOFF;
                let _size = reader.read_u16::<LittleEndian>()?;
                let diffuse = if (mat_type & 1) != 0 {
                    Some(Color::from_buf(reader)?)
                } else {
                    None
                };

                let ambient = if (mat_type & 2) != 0 {
                    Some(Color::from_buf(reader)?)
                } else {
                    None
                };

                let specular = if (mat_type & 4) != 0 {
                    Some(Color::from_buf(reader)?)
                } else {
                    None
                };

                Ok(Some(PolyChunk::Material {
                    source_alpha: AlphaInstruction::try_from((plist_flags >> 3) & 7).unwrap(),
                    destination_alpha: AlphaInstruction::try_from(plist_flags & 7).unwrap(),
                    diffuse,
                    ambient,
                    specular,
                }))
            }

            NJD_STRIPOFF..NJD_STRIPOFF_END => {
                let _size = reader.read_u16::<LittleEndian>()?;
                let header2 = reader.read_u16::<LittleEndian>()?;

                let strip_count = (header2 & 0x3FFF) as usize;
                let user_flags_count = (header2 >> 14) as u8;
                let mut strips: Vec<Strip> = Vec::with_capacity(strip_count);

                for _ in 0..strip_count {
                    strips.push(Strip::from_buf(
                        reader,
                        PolyChunkType::try_from(plist_type as u8).unwrap(),
                        user_flags_count as u32,
                    )?);
                }

                Ok(Some(PolyChunk::Strip {
                    flags: plist_flags,
                    user_flags: user_flags_count,
                    strips,
                }))
            }

            NJD_ENDOFF => Ok(None),

            _ => Ok(None),
        };

        let end_list = reader.stream_position().unwrap();

        reader.rewind()?;

        let plist_len = (end_list - plist_start) as usize;

        reader.seek(SeekFrom::Start(plist_start))?;

        let mut real_bytes = vec![0u8; plist_len];
        reader.read_exact(&mut real_bytes)?;

        reader.seek(SeekFrom::Start(end_list))?;

        let plist = plist?;

        Ok(plist.map(|x| (real_bytes.clone(), x)))
    }
}
