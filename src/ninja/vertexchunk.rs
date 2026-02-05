use super::{
    error::NinjaParseError,
    math::{Color, Point3},
};
use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    io::{Read, Seek, SeekFrom},
    u8,
};
use zerocopy::IntoBytes;

#[derive(Clone, Copy, Debug)]
pub enum VertexChunkType {
    SH,
    NormalSH,
    Vertex,
    Diffuse8,
    UserFlags,
    NinjaFlags,
    DiffuseSpecular5,
    DiffuseSpecular4,
    DiffuseSpecular16,
    Normal,
    NormalDiffuse8,
    NormalUserFlags,
    NormalNinjaFlags,
    NormalDiffuseSpecular5,
    NormalDiffuseSpecular4,
    NormalDiffuseSpecular16,
    NormalX,
    NormalXDiffuse8,
    NormalXUserFlags,
    End,
}

impl VertexChunkType {
    fn is_sh(&self) -> bool {
        match self {
            VertexChunkType::SH => true,
            VertexChunkType::NormalSH => true,
            _ => false,
        }
    }

    fn has_normal(&self) -> bool {
        match self {
            VertexChunkType::NormalSH => true,
            VertexChunkType::Normal => true,
            VertexChunkType::NormalDiffuse8 => true,
            VertexChunkType::NormalUserFlags => true,
            VertexChunkType::NormalNinjaFlags => true,
            VertexChunkType::NormalDiffuseSpecular5 => true,
            VertexChunkType::NormalDiffuseSpecular4 => true,
            VertexChunkType::NormalDiffuseSpecular16 => true,
            VertexChunkType::NormalX => true,
            VertexChunkType::NormalXDiffuse8 => true,
            VertexChunkType::NormalXUserFlags => true,
            _ => false,
        }
    }

    fn has_diffuse(&self) -> bool {
        match self {
            VertexChunkType::Diffuse8 => true,
            VertexChunkType::DiffuseSpecular5 => true,
            VertexChunkType::DiffuseSpecular4 => true,
            VertexChunkType::DiffuseSpecular16 => true,
            VertexChunkType::NormalDiffuse8 => true,
            VertexChunkType::NormalDiffuseSpecular5 => true,
            VertexChunkType::NormalDiffuseSpecular4 => true,
            VertexChunkType::NormalDiffuseSpecular16 => true,
            VertexChunkType::NormalXDiffuse8 => true,
            _ => false,
        }
    }

    fn has_user_flags(&self) -> bool {
        match self {
            VertexChunkType::UserFlags => true,
            VertexChunkType::NormalUserFlags => true,
            VertexChunkType::NormalXUserFlags => true,
            _ => false,
        }
    }

    fn has_ninja_flags(&self) -> bool {
        match self {
            VertexChunkType::NinjaFlags => true,
            VertexChunkType::NormalNinjaFlags => true,
            _ => false,
        }
    }
}

impl TryFrom<u8> for VertexChunkType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            32 => Ok(VertexChunkType::SH),
            33 => Ok(VertexChunkType::NormalSH),
            34 => Ok(VertexChunkType::Vertex),
            35 => Ok(VertexChunkType::Diffuse8),
            36 => Ok(VertexChunkType::UserFlags),
            37 => Ok(VertexChunkType::NinjaFlags),
            38 => Err(()),
            39 => Err(()),
            40 => Err(()),
            41 => Ok(VertexChunkType::Normal),
            42 => Ok(VertexChunkType::NormalDiffuse8),
            43 => Ok(VertexChunkType::NormalUserFlags),
            44 => Ok(VertexChunkType::NormalNinjaFlags),
            45 => Err(()),
            46 => Err(()),
            47 => Err(()),
            48 => Ok(VertexChunkType::NormalX),
            49 => Ok(VertexChunkType::NormalXDiffuse8),
            50 => Ok(VertexChunkType::NormalXUserFlags),
            255 => Ok(VertexChunkType::End),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum WeightStatus {
    Start,
    Middle,
    End,
}

impl TryFrom<u8> for WeightStatus {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(WeightStatus::Start),
            1 => Ok(WeightStatus::Middle),
            2 => Ok(WeightStatus::End),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VertexChunk {
    pub chunk_type: VertexChunkType,
    pub weight_status: Option<WeightStatus>,
    pub index_offset: u16,
    pub vertex_count: u32,

    header1: u32,
    header2: u32,
    size: u16,

    pub vertices: Vec<Point3>,
    pub normals: Option<Vec<Point3>>,
    pub diffuse: Option<Vec<u32>>,
    pub specular: Option<Vec<Color>>,
    pub user_flags: Option<Vec<u32>>,
    pub ninja_flags: Option<Vec<u32>>,
}

impl VertexChunk {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(self.header1.as_bytes());
        bytes.extend_from_slice(self.header2.as_bytes());
        for i in 0..self.vertex_count as usize {
            bytes.extend_from_slice(self.vertices[i].x.as_bytes());
            bytes.extend_from_slice(self.vertices[i].y.as_bytes());
            bytes.extend_from_slice(self.vertices[i].z.as_bytes());
            if self.chunk_type.is_sh() {
                bytes.extend_from_slice(1.0.as_bytes());
            }

            if let Some(normals) = &self.normals {
                bytes.extend_from_slice(normals[i].x.as_bytes());
                bytes.extend_from_slice(normals[i].y.as_bytes());
                bytes.extend_from_slice(normals[i].z.as_bytes());
                if self.chunk_type.is_sh() {
                    bytes.extend_from_slice(1.0.as_bytes());
                }
            }

            if let Some(diffuse) = &self.diffuse {
                bytes.extend_from_slice(diffuse[i].as_bytes());
            }

            if let Some(user_flags) = &self.user_flags {
                bytes.extend_from_slice(user_flags[i].as_bytes());
            }

            if let Some(ninja_flags) = &self.ninja_flags {
                bytes.extend_from_slice(ninja_flags[i].as_bytes());
            }
        }
        bytes
    }

    pub fn from_buf<R: Read + Seek>(reader: &mut R) -> Result<Option<Self>, NinjaParseError> {
        let header1 = reader.read_u32::<LittleEndian>()?;
        let header2 = reader.read_u32::<LittleEndian>()?;

        let chunk_type = VertexChunkType::try_from((header1 & 0xFF) as u8).unwrap();

        if let VertexChunkType::End = chunk_type {
            return Ok(None);
        }

        let flags = ((header1 >> 8) & 0xFF) as u8;
        let size = (header1 >> 16) as u16;

        let mut weight_status = None;

        if chunk_type.has_ninja_flags() {
            weight_status = Some(WeightStatus::try_from((flags & 3) as u8).unwrap());
        }

        let index_offset = (header2 & 0xFFFF) as u16;
        let vertex_count = header2 >> 16;

        let mut vertices: Vec<Point3> = Vec::new();
        let mut normals: Option<Vec<Point3>> = if chunk_type.has_normal() {
            Some(Vec::new())
        } else {
            None
        };
        let mut diffuse: Option<Vec<u32>> = if chunk_type.has_diffuse() {
            Some(Vec::new())
        } else {
            None
        };
        //let mut specular: Option<Vec<Color>> = if chunk_type.has_specular() { Some(Vec::new()) } else { None };
        let mut user_flags: Option<Vec<u32>> = if chunk_type.has_user_flags() {
            Some(Vec::new())
        } else {
            None
        };
        let mut ninja_flags: Option<Vec<u32>> = if chunk_type.has_ninja_flags() {
            Some(Vec::new())
        } else {
            None
        };

        for _ in 0..vertex_count {
            vertices.push(Point3::from_buf(reader)?);

            if chunk_type.is_sh() {
                reader.seek(SeekFrom::Current(4))?;
            }

            if let Some(ref mut vec) = normals {
                vec.push(Point3::from_buf(reader)?);

                if chunk_type.is_sh() {
                    reader.seek(SeekFrom::Current(4))?;
                }
            }

            if let Some(ref mut vec) = diffuse {
                vec.push(reader.read_u32::<LittleEndian>()?);
            }

            if let Some(ref mut vec) = user_flags {
                vec.push(reader.read_u32::<LittleEndian>()?);
            }

            if let Some(ref mut vec) = ninja_flags {
                vec.push(reader.read_u32::<LittleEndian>()?);
            }
        }

        Ok(Some(VertexChunk {
            chunk_type,
            weight_status,
            size,
            index_offset,
            vertex_count,
            header1,
            header2,
            vertices,
            normals,
            diffuse,
            specular: None,
            user_flags,
            ninja_flags,
        }))
    }
}
