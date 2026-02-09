use std::{
    fs,
    hash::Hasher,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use byteorder::{LittleEndian, ReadBytesExt};
use twox_hash::xxhash32;

use super::{
    chunkmodel::ChunkModel,
    error::NinjaParseError,
    labels::SAFileLabels,
    math::{NinjaRotation, Point3},
};

const CURRENT_VERSION: u8 = 3;
const FORMAT_MASK: u64 = 0xFFFFFFFFFFFFFFu64;

const CHUNK_FORMAT: u64 = 0x4C444D324153u64;

#[derive(Clone)]
pub struct NinjaChunkObject {
    pub name: String,
    pub eval_flags: u32,
    pub model: Option<ChunkModel>,
    pub pos: Point3,
    pub ang: NinjaRotation,
    pub scl: Point3,

    pub child: Option<Box<NinjaChunkObject>>,
    pub sibling: Option<Box<NinjaChunkObject>>,
}

impl NinjaChunkObject {
    fn find_hierarchy_vlist_plist(&self, vlist: &mut Option<Vec<u8>>, plist: &mut Option<Vec<u8>>) {
        if plist.is_some() && vlist.is_some() {
            return;
        }

        if let Some(model) = &self.model {
            if !model.vertex_list.is_empty() && vlist.is_none() {
                let mut vlist_bytes: Vec<u8> = model
                    .vertex_list
                    .iter()
                    .flat_map(|v| v.to_bytes())
                    .collect();
                vlist_bytes.extend_from_slice(&[0xFF, 0x00, 0x00, 0x00]);
                *vlist = Some(vlist_bytes);
            }

            if !model.poly_bytes.is_empty() && plist.is_none() {
                let mut plist_bytes: Vec<u8> = model.poly_bytes.clone();
                plist_bytes.extend_from_slice(&[0xFF, 0x00]);
                *plist = Some(plist_bytes);
            }
        }

        if let Some(child) = &self.child {
            child.find_hierarchy_vlist_plist(vlist, plist);
        }

        if let Some(sibling) = &self.sibling {
            sibling.find_hierarchy_vlist_plist(vlist, plist);
        }
    }

    pub fn get_hash(&self) -> Option<u32> {
        let mut vlist = None;
        let mut plist = None;
        self.find_hierarchy_vlist_plist(&mut vlist, &mut plist);

        if vlist.is_none() || plist.is_none() {
            return None;
        }

        let mut hasher = xxhash32::Hasher::with_seed(0);
        hasher.write(vlist.unwrap().as_slice());
        hasher.write(plist.unwrap().as_slice());

        Some(hasher.finish_32())
    }

    pub fn export_c(&self, writer: &mut dyn std::fmt::Write) -> Result<String, std::fmt::Error> {
        let child_name = if let Some(child) = &self.child {
            "&".to_string() + &child.export_c(writer)?
        } else {
            "NULL".to_string()
        };

        let sibling_name = if let Some(sibling) = &self.sibling {
            "&".to_string() + &sibling.export_c(writer)?
        } else {
            "NULL".to_string()
        };

        let model_name = if let Some(model) = &self.model {
            "&".to_string() + &model.export_c(writer)?
        } else {
            "NULL".to_string()
        };

        writer.write_str(
            format!(
                "NJS_OBJECT {} = {{{}, {}, {{{}, {}, {}}}, {{{}, {}, {}}}, {{{}, {}, {}}}, {}, {} }};\n\n",
                self.name,
                self.eval_flags,
                model_name,
                self.pos.x,
                self.pos.y,
                self.pos.z,
                self.ang.x,
                self.ang.y,
                self.ang.z,
                self.scl.x,
                self.scl.y,
                self.scl.z,
                child_name,
                sibling_name
            )
            .as_str(),
        )?;

        let name = "&".to_owned() + &self.name;
        Ok(name)
    }

    fn get_node_count_sub(&self, counter: &mut usize) {
        *counter += 1;

        if let Some(child) = &self.child {
            child.get_node_count_sub(counter);
        }

        if let Some(sibling) = &self.sibling {
            sibling.get_node_count_sub(counter);
        }
    }

    pub fn get_node_count(&self) -> usize {
        let mut counter = 0;
        self.get_node_count_sub(&mut counter);
        counter
    }

    pub fn get_node(&mut self, counter: &mut usize, index: usize) -> Option<&mut NinjaChunkObject> {
        if *counter == index {
            return Some(self);
        }

        *counter += 1;

        if let Some(child) = &mut self.child {
            if let Some(result) = child.get_node(counter, index) {
                return Some(result);
            }
        }

        if let Some(sibling) = &mut self.sibling {
            if let Some(result) = sibling.get_node(counter, index) {
                return Some(result);
            }
        }

        None
    }

    fn from_buf<R: Read + Seek>(
        reader: &mut R,
        labels: &SAFileLabels,
        obj_start: u64,
        obj_base: u64,
    ) -> Result<Box<Self>, NinjaParseError> {
        reader.seek(SeekFrom::Start(obj_start))?;

        let eval_flags = reader.read_u32::<LittleEndian>()?;
        let model_ptr = u64::from(reader.read_u32::<LittleEndian>()?) - obj_base;
        let pos_x = reader.read_f32::<LittleEndian>()?;
        let pos_y = reader.read_f32::<LittleEndian>()?;
        let pos_z = reader.read_f32::<LittleEndian>()?;

        let ang_x = reader.read_i32::<LittleEndian>()?;
        let ang_y = reader.read_i32::<LittleEndian>()?;
        let ang_z = reader.read_i32::<LittleEndian>()?;

        let scl_x = reader.read_f32::<LittleEndian>()?;
        let scl_y = reader.read_f32::<LittleEndian>()?;
        let scl_z = reader.read_f32::<LittleEndian>()?;

        let child_ptr = u64::from(reader.read_u32::<LittleEndian>()?) - obj_base;
        let sibling_ptr = u64::from(reader.read_u32::<LittleEndian>()?) - obj_base;

        let mut chunkmodel = None;
        let mut child = None;
        let mut sibling = None;

        if model_ptr != 0 {
            chunkmodel = Some(ChunkModel::from_buf(reader, model_ptr, obj_base)?);
        }

        if child_ptr != 0 {
            child = Some(NinjaChunkObject::from_buf(
                reader,
                labels,
                child_ptr - obj_base,
                obj_base,
            )?);
        }

        if sibling_ptr != 0 {
            sibling = Some(NinjaChunkObject::from_buf(
                reader,
                labels,
                sibling_ptr - obj_base,
                obj_base,
            )?);
        }

        let obj = Box::new(NinjaChunkObject {
            name: labels.get_name(&obj_start, "object"),
            eval_flags,
            pos: Point3 {
                x: pos_x,
                y: pos_y,
                z: pos_z,
            },
            ang: NinjaRotation {
                x: ang_x,
                y: ang_y,
                z: ang_z,
            },
            scl: Point3 {
                x: scl_x,
                y: scl_y,
                z: scl_z,
            },
            model: chunkmodel,
            child,
            sibling,
        });

        Ok(obj)
    }

    pub fn new(buf: &[u8]) -> Result<Box<Self>, NinjaParseError> {
        let mut rdr = Cursor::new(buf);

        let magic = rdr.read_u64::<LittleEndian>()?;
        let version = u8::try_from(magic >> 56).unwrap();

        if version != CURRENT_VERSION {
            return Err(NinjaParseError::FormatError("Invalid version!"));
        }

        if (magic & FORMAT_MASK) != CHUNK_FORMAT {
            return Err(NinjaParseError::FormatError("Unsupported format!"));
        }

        let model_off = rdr.read_u32::<LittleEndian>()? as u64;
        let label_off = rdr.read_u32::<LittleEndian>()? as u64;

        rdr.seek(SeekFrom::Start(label_off))?;
        let labels = SAFileLabels::read(&mut rdr).map_err(|_| NinjaParseError::IoError())?;

        Self::from_buf(&mut rdr, &labels, model_off, 0)
    }

    pub fn read_file(path: &PathBuf) -> Result<Box<Self>, NinjaParseError> {
        let data: Vec<u8> = fs::read(path)?;
        Self::new(&data)
    }
}
