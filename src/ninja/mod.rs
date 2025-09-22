pub mod anim;
pub mod chunkmodel;
pub mod error;
mod labels;
pub mod math;
pub mod modelfile;
pub mod ninjadraw;
pub mod ninjamatrix;
pub mod polychunk;
pub mod texlist;
pub mod texture;
pub mod vertexchunk;

#[derive(Debug, Clone, Copy)]
pub enum FilterMode {
    PointSampled,
    Bilinear,
    Trilinear,
    Reserved,
}

#[derive(Debug, Clone, Copy)]
pub enum AlphaInstruction {
    Zero,
    One,
    OtherColor,
    InverseOtherColor,
    SourceAlpha,
    InverseSourceAlpha,
    DestinationAlpha,
    InverseDestinationAlpha,
}

impl Into<wgpu::BlendFactor> for AlphaInstruction {
    fn into(self) -> wgpu::BlendFactor {
        match self {
            AlphaInstruction::Zero => wgpu::BlendFactor::Zero,
            AlphaInstruction::One => wgpu::BlendFactor::One,
            AlphaInstruction::OtherColor => wgpu::BlendFactor::Src1,
            AlphaInstruction::InverseOtherColor => wgpu::BlendFactor::OneMinusSrc1,
            AlphaInstruction::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
            AlphaInstruction::InverseSourceAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
            AlphaInstruction::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
            AlphaInstruction::InverseDestinationAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        }
    }
}

impl TryFrom<u8> for FilterMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FilterMode::PointSampled),
            1 => Ok(FilterMode::Bilinear),
            2 => Ok(FilterMode::Trilinear),
            3 => Ok(FilterMode::Reserved),
            _ => Err(()),
        }
    }
}

impl Into<u8> for AlphaInstruction {
    fn into(self) -> u8 {
        match self {
            AlphaInstruction::Zero => 0,
            AlphaInstruction::One => 1,
            AlphaInstruction::OtherColor => 2,
            AlphaInstruction::InverseOtherColor => 3,
            AlphaInstruction::SourceAlpha => 4,
            AlphaInstruction::InverseSourceAlpha => 5,
            AlphaInstruction::DestinationAlpha => 6,
            AlphaInstruction::InverseDestinationAlpha => 7,
        }
    }
}

impl TryFrom<u8> for AlphaInstruction {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AlphaInstruction::Zero),
            1 => Ok(AlphaInstruction::One),
            2 => Ok(AlphaInstruction::OtherColor),
            3 => Ok(AlphaInstruction::InverseOtherColor),
            4 => Ok(AlphaInstruction::SourceAlpha),
            5 => Ok(AlphaInstruction::InverseSourceAlpha),
            6 => Ok(AlphaInstruction::DestinationAlpha),
            7 => Ok(AlphaInstruction::InverseDestinationAlpha),
            _ => Err(()),
        }
    }
}
