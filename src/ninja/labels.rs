use byteorder::{LittleEndian, ReadBytesExt};
use std::{
    collections::HashMap,
    ffi::CStr,
    io::{Read, Seek, SeekFrom},
};

pub enum SAFileLabelError {
    IoError,
}

impl From<std::io::Error> for SAFileLabelError {
    fn from(_: std::io::Error) -> Self {
        Self::IoError
    }
}

pub struct SAFileLabels {
    map: HashMap<u64, String>,
}

impl SAFileLabels {
    pub fn get_name(&self, ptr: &u64, fallback_prefix: &'static str) -> String {
        if self.map.contains_key(ptr) {
            return self.map[ptr].clone();
        }

        format!("{}_{:#x}", fallback_prefix, ptr)
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, SAFileLabelError> {
        const CHUNK_LABEL: u32 = 0x4C42414C;
        const CHUNK_END: u32 = 0x444E45;

        let mut map = HashMap::new();

        let mut chunk_type = reader.read_u32::<LittleEndian>()?;
        while chunk_type != CHUNK_END {
            let size = reader.read_u32::<LittleEndian>()? as u64;
            let chunk_pos = reader.stream_position()?;

            if chunk_type == CHUNK_LABEL {
                loop {
                    let data_ptr = reader.read_u32::<LittleEndian>()? as u64;
                    let label_ptr = reader.read_u32::<LittleEndian>()? as u64;

                    if data_ptr == u32::MAX as u64 && label_ptr == u32::MAX as u64 {
                        break;
                    }

                    let pos = reader.stream_position()?;
                    reader.seek(SeekFrom::Start(chunk_pos + label_ptr))?;
                    let mut buffer = Vec::new();
                    loop {
                        let mut character: [u8; 1] = [0];
                        reader.read_exact(&mut character)?;
                        buffer.push(character[0]);

                        if character[0] == 0 {
                            break;
                        }
                    }

                    let string = CStr::from_bytes_until_nul(&buffer).unwrap();

                    map.insert(data_ptr, string.to_str().unwrap().to_string());

                    reader.seek(std::io::SeekFrom::Start(pos))?;
                }
            }

            reader.seek(std::io::SeekFrom::Start(chunk_pos + size))?;
            chunk_type = reader.read_u32::<LittleEndian>()?;
        }

        Ok(Self { map })
    }
}
