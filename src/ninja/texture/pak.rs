use std::io::{Cursor, Read, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

use super::error::NinjaTexReadError;

#[derive(Debug)]
pub struct PAKEntry {
    data: Vec<u8>,
    name: String,
    path: String,
}

impl PAKEntry {
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_path(&self) -> &String {
        &self.path
    }
}

#[derive(Debug)]
pub struct PAKFile {
    pub file_entries: Vec<PAKEntry>,
}

impl PAKFile {
    pub fn find_file(&self, name: &String) -> Option<&PAKEntry> {
        self.file_entries.iter().find(|x| x.get_name().eq(name))
    }

    pub fn load_pak(buf: &[u8]) -> Result<Self, NinjaTexReadError> {
        let mut reader = Cursor::new(buf);

        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;

        if header[0] == 0x1 && header[1..] != *b"pak" {
            return Err(NinjaTexReadError::FormatError("PAK header invalid"));
        }

        reader.seek(SeekFrom::Current(0x35))?;

        let num_entries = reader.read_u32::<LittleEndian>()? as usize;

        let mut entries = Vec::new();
        let mut names = vec![String::new(); num_entries];
        let mut paths = vec![String::new(); num_entries];
        let mut lengths = vec![0usize; num_entries];

        for i in 0..num_entries {
            let len_path = reader.read_u32::<LittleEndian>()? as usize;
            let mut path = vec![0u8; len_path];
            reader.read_exact(&mut path)?;
            paths[i] = String::from_utf8(path)?;

            let len_name = reader.read_u32::<LittleEndian>()? as usize;
            let mut name = vec![0u8; len_name];
            reader.read_exact(&mut name)?;
            names[i] = String::from_utf8(name)?;

            lengths[i] = reader.read_u32::<LittleEndian>()? as usize;
            reader.seek(SeekFrom::Current(4))?;
        }

        for i in 0..num_entries {
            let mut buf = vec![0u8; lengths[i]];
            reader.read_exact(&mut buf)?;
            entries.push(PAKEntry {
                data: buf,
                name: names[i].clone(),
                path: paths[i].clone(),
            });
        }

        Ok(PAKFile {
            file_entries: entries,
        })
    }
}
