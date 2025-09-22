use std::{
    i32,
    io::{Cursor, Read, Seek, SeekFrom},
    u8,
};

use byteorder::{LittleEndian, ReadBytesExt};

use super::{
    error::NinjaParseError,
    math::{NinjaRotation, Point3},
};

type NinjaKeyframe<T> = (usize, T);

#[derive(Clone)]
struct NinjaKeyframeList<T> {
    list: Vec<NinjaKeyframe<T>>,
}

pub trait NinjaKeyframeInterpolation<T> {
    fn interpolate(&self, frame: f32) -> T;
}

impl<T> NinjaKeyframeList<T> {
    fn search_key(&self, frame: usize) -> usize {
        let mut nb_a = 0usize;
        let mut nb_b = self.list.len();

        while (nb_b - nb_a) > 1 {
            let nb_mid = (nb_b + nb_a) / 2;

            if frame >= self.list[nb_mid].0 {
                nb_a = nb_mid;
            } else {
                nb_b = nb_mid;
            }
        }

        nb_a
    }

    fn get_linear_keys(&self, frame: f32) -> (usize, usize, f32) {
        let index = self.search_key(frame as usize);
        let key1 = index;
        let key2 = if index < self.list.len() - 1 {
            index + 1
        } else {
            index
        };

        let frame1 = self.list[key1].0 as f32;
        let frame2 = self.list[key2].0 as f32;
        let diff = frame2 - frame1;

        (
            key1,
            key2,
            if diff > 0.0 {
                (frame - frame1) / diff
            } else {
                frame - frame1
            },
        )
    }
}

type NinjaPoint3KeyframeList = NinjaKeyframeList<Point3>;
type NinjaRotationKeyframeList = NinjaKeyframeList<NinjaRotation>;

impl NinjaKeyframeInterpolation<Point3> for NinjaPoint3KeyframeList {
    fn interpolate(&self, frame: f32) -> Point3 {
        let (index1, index2, rate) = self.get_linear_keys(frame);
        self.list[index1].1.lerp(&self.list[index2].1, rate)
    }
}

impl NinjaKeyframeInterpolation<NinjaRotation> for NinjaRotationKeyframeList {
    fn interpolate(&self, frame: f32) -> NinjaRotation {
        let (index1, index2, rate) = self.get_linear_keys(frame);
        self.list[index1].1.lerp(&self.list[index2].1, rate)
    }
}

#[derive(Clone)]
pub struct NinjaMotion {
    pos: Option<Vec<Option<NinjaPoint3KeyframeList>>>,
    ang: Option<Vec<Option<NinjaRotationKeyframeList>>>,
    scl: Option<Vec<Option<NinjaPoint3KeyframeList>>>,

    nb_frame: usize,
    model_count: usize,
}

impl NinjaMotion {
    pub fn get_num_frames(&self) -> usize {
        self.nb_frame
    }

    fn read_float_frames<R: Read>(
        reader: &mut R,
        nb_frame: usize,
    ) -> Result<NinjaPoint3KeyframeList, NinjaParseError> {
        let mut list = Vec::new();

        for _ in 0..nb_frame {
            let keyframe = reader.read_u32::<LittleEndian>()? as usize;
            let value = Point3::from_buf(reader)?;
            list.push((keyframe, value));
        }

        Ok(NinjaKeyframeList { list })
    }

    pub fn get_motion_pos(&self, model_index: usize, frame: f32) -> Option<Point3> {
        self.pos
            .as_ref()
            .and_then(|pos| pos[model_index].as_ref())
            .and_then(|frames| Some(frames.interpolate(frame)))
    }

    pub fn get_motion_ang(&self, model_index: usize, frame: f32) -> Option<NinjaRotation> {
        self.ang
            .as_ref()
            .and_then(|ang| ang[model_index].as_ref())
            .and_then(|frames| Some(frames.interpolate(frame)))
    }

    pub fn get_motion_scl(&self, model_index: usize, frame: f32) -> Option<Point3> {
        self.scl
            .as_ref()
            .and_then(|scl| scl[model_index].as_ref())
            .and_then(|frames| Some(frames.interpolate(frame)))
    }

    fn read_ang_frames<R: Read>(
        reader: &mut R,
        nb_frame: usize,
        short_rot: bool,
    ) -> Result<NinjaRotationKeyframeList, NinjaParseError> {
        let mut list = Vec::new();

        for _ in 0..nb_frame {
            let (keyframe, value) = if !short_rot {
                (
                    reader.read_u32::<LittleEndian>()? as usize,
                    NinjaRotation::from_buf(reader)?,
                )
            } else {
                (
                    reader.read_u16::<LittleEndian>()? as usize,
                    NinjaRotation::from_buf_16(reader)?,
                )
            };
            list.push((keyframe, value));
        }

        Ok(NinjaKeyframeList { list })
    }

    pub fn new(buf: &[u8]) -> Result<Self, NinjaParseError> {
        const SAANIM_MAGIC: u64 = 0x4D494E414153;
        const FORMAT_MASK: u64 = 0xFFFFFFFFFFFF;
        const CURRENT_VERSION: u8 = 2;
        //const int headersize = 0x14;
        let mut cursor = Cursor::new(buf);
        let magic = cursor.read_u64::<LittleEndian>()?;
        let version = u8::try_from(magic >> 56)
            .map_err(|_| NinjaParseError::FormatError("Error parsing magic number!"))?;

        if version > CURRENT_VERSION {
            return Err(NinjaParseError::FormatError(
                "Invalid animation file version!",
            ));
        }

        if (magic & FORMAT_MASK) != SAANIM_MAGIC {
            return Err(NinjaParseError::FormatError("Incorrect animation magic!"));
        }

        let motion_offset = cursor.read_u32::<LittleEndian>()? as u64;
        let _label_offset = cursor.read_u32::<LittleEndian>()? as u64;
        let model_count_signed = cursor.read_i32::<LittleEndian>()?;
        // chao use motions that don't store properly if they're shortrot or not
        // this is a hack in the saanim format to signify that
        let short_rot = model_count_signed < 0;
        let model_count = (model_count_signed & i32::MAX) as usize;

        cursor.seek(SeekFrom::Start(motion_offset))?;

        let mdata_offset = cursor.read_u32::<LittleEndian>()? as u64;
        let nb_frame = cursor.read_u32::<LittleEndian>()? as usize;
        let _motion_type = cursor.read_u16::<LittleEndian>()?;
        let mdata_entrycount = (cursor.read_u16::<LittleEndian>()? & 0xF) as u64;

        let has_pos = mdata_entrycount >= 1;
        let has_ang = mdata_entrycount >= 2;
        let has_scl = mdata_entrycount >= 3;

        let mut pos = Vec::new();
        let mut ang = Vec::new();
        let mut scl = Vec::new();

        if mdata_entrycount > 3 {
            return Err(NinjaParseError::FormatError("Unsupported MDATA type!"));
        }

        for i in 0..model_count {
            let start_offset = mdata_offset + 8 * mdata_entrycount * (i as u64);

            let read_frame_pointer = |cursor: &mut Cursor<&[u8]>,
                                      offset: u64,
                                      mdata_index|
             -> Result<(u32, usize), NinjaParseError> {
                cursor.seek(SeekFrom::Start(offset + mdata_index * 4))?;
                let pointer = cursor.read_u32::<LittleEndian>()?;
                cursor.seek(SeekFrom::Start(
                    offset + (mdata_entrycount + mdata_index) * 4,
                ))?;
                let frame = cursor.read_u32::<LittleEndian>()? as usize;
                Ok((pointer, frame))
            };

            if has_pos {
                let (offset, frame) = read_frame_pointer(&mut cursor, start_offset, 0)?;
                if offset > 0 {
                    cursor.seek(SeekFrom::Start(offset.into()))?;
                    pos.push(Some(Self::read_float_frames(&mut cursor, frame)?));
                } else {
                    pos.push(None);
                }
            }

            if has_ang {
                let (offset, frame) = read_frame_pointer(&mut cursor, start_offset, 1)?;
                if offset > 0 {
                    cursor.seek(SeekFrom::Start(offset.into()))?;
                    ang.push(Some(Self::read_ang_frames(&mut cursor, frame, short_rot)?));
                } else {
                    ang.push(None);
                }
            }

            if has_scl {
                let (offset, frame) = read_frame_pointer(&mut cursor, start_offset, 2)?;
                if offset > 0 {
                    cursor.seek(SeekFrom::Start(offset.into()))?;
                    scl.push(Some(Self::read_float_frames(&mut cursor, frame)?));
                } else {
                    scl.push(None);
                }
            }
        }

        Ok(Self {
            pos: has_pos.then(|| pos),
            ang: has_ang.then(|| ang),
            scl: has_scl.then(|| scl),
            nb_frame,
            model_count,
        })
    }
}
