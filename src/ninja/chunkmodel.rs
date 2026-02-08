use byteorder::{LittleEndian, ReadBytesExt};
use rand::distr::{Alphanumeric, SampleString};
use std::collections::HashMap;
use std::fmt::Write;
use std::io::{Read, Seek, SeekFrom};

use crate::ninja::polychunk::PolyChunk;

use super::vertexchunk::VertexChunk;
use super::{error::NinjaParseError, math::Point3};

#[derive(Clone)]
pub struct ChunkModel {
    pub vertex_list: Vec<VertexChunk>,
    pub poly_list: Option<Vec<PolyChunk>>,
    pub poly_bytes: Vec<u8>,
    pub _center: Point3,
    pub _r: f32,
}

impl ChunkModel {
    pub fn export_c(&self, writer: &mut dyn Write) -> Result<String, std::fmt::Error> {
        let vertex_chunk_name =
            "vertex_".to_string() + &Alphanumeric.sample_string(&mut rand::rng(), 16);
        let poly_chunk_name =
            "poly_".to_string() + &Alphanumeric.sample_string(&mut rand::rng(), 16);

        let model_name = "model_".to_string() + &Alphanumeric.sample_string(&mut rand::rng(), 16);

        let poly_vec: Vec<u16> = Vec::from_iter(
            self.poly_bytes
                .chunks(2)
                .map(|x| zerocopy::byteorder::little_endian::U16::from_bytes([x[0], x[1]]).get()),
        );

        if !poly_vec.is_empty() {
            writer.write_str(format!("Sint16 {}[] = {{", poly_chunk_name).as_str())?;
            poly_vec.iter().cloned().for_each(|x| {
                let v = x;
                let str = format!("{:#02x}, ", v);
                writer.write_str(str.as_str());
            });
            writer.write_str("};\n\n")?;
        }

        let mut vertex_chunk_bytes: Vec<u8> =
            self.vertex_list.iter().flat_map(|v| v.to_bytes()).collect();
        vertex_chunk_bytes.extend_from_slice(&[255, 255, 255, 255]);

        let vertex_vec: Vec<u32> = Vec::from_iter(vertex_chunk_bytes.chunks(4).map(|x| {
            zerocopy::byteorder::little_endian::U32::from_bytes([x[0], x[1], x[2], x[3]]).get()
        }));

        if !vertex_vec.is_empty() {
            writer.write_str(format!("Sint32 {}[] = {{", vertex_chunk_name).as_str())?;
            for v in &vertex_vec {
                let str = format!("{:#04x}, ", v);
                writer.write_str(str.as_str())?;
            }
            writer.write_str("}};\n\n")?;
        }

        let vertex_reference = if vertex_vec.is_empty() {
            "NULL"
        } else {
            &("&".to_string() + &vertex_chunk_name)
        };
        let poly_reference = if poly_vec.is_empty() {
            "NULL"
        } else {
            &("&".to_string() + &poly_chunk_name)
        };
        writer.write_str(
            format!(
                "NJS_CNK_MODEL {} = {{{}, {}, {{{}, {}, {}}}, {}}};\n\n",
                model_name,
                vertex_reference,
                poly_reference,
                self._center.x,
                self._center.y,
                self._center.z,
                self._r
            )
            .as_str(),
        )?;

        Ok(model_name)
    }

    pub fn get_face_adjacency(&self) -> HashMap<(usize, usize, usize), Vec<(usize, usize, usize)>> {
        let mut map: HashMap<(usize, usize), Vec<(usize, usize, usize)>> = HashMap::new();

        if let Some(poly_list) = &self.poly_list {
            for (poly_index, poly) in poly_list.iter().enumerate() {
                match poly {
                    PolyChunk::Strip {
                        flags: _,
                        user_flags: _,
                        strips,
                    } => {
                        for (strip_index, strip) in strips.iter().enumerate() {
                            let skip_one = strip.indices.iter().skip(1);
                            let skip_two = strip.indices.iter().skip(2);
                            for (index, ((a, b), c)) in
                                strip.indices.iter().zip(skip_one).zip(skip_two).enumerate()
                            {
                                for (x, y) in [(a, b), (b, c), (c, a)] {
                                    let index1 = *x as usize;
                                    let index2 = *y as usize;
                                    let value = (poly_index, strip_index, index);

                                    if map.contains_key(&(index1, index2)) {
                                        map.get_mut(&(index1, index2))
                                            .as_mut()
                                            .unwrap()
                                            .push(value);
                                    } else if map.contains_key(&(index2, index1)) {
                                        map.get_mut(&(index2, index1))
                                            .as_mut()
                                            .unwrap()
                                            .push(value);
                                    } else {
                                        map.insert((index1, index2), vec![value]);
                                    };
                                }
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        let mut adjacency: HashMap<(usize, usize, usize), Vec<(usize, usize, usize)>> =
            HashMap::new();
        map.iter().for_each(|((_, _), list)| {
            if list.len() == 2 {
                if let Some(list0) = adjacency.get_mut(&(list[0])) {
                    list0.push(list[1]);
                } else {
                    adjacency.insert(list[0], vec![list[1]]);
                }

                if let Some(list1) = adjacency.get_mut(&(list[1])) {
                    list1.push(list[0]);
                } else {
                    adjacency.insert(list[1], vec![list[0]]);
                }
            }
        });

        adjacency
    }

    pub fn from_buf<R: Read + Seek>(
        reader: &mut R,
        mdl_start: u64,
        obj_base: u64,
    ) -> Result<Self, NinjaParseError> {
        reader.seek(SeekFrom::Start(mdl_start))?;

        let vlist_ptr = u64::from(reader.read_u32::<LittleEndian>()?) - obj_base;
        let plist_ptr = u64::from(reader.read_u32::<LittleEndian>()?) - obj_base;
        let center_x = reader.read_f32::<LittleEndian>()?;
        let center_y = reader.read_f32::<LittleEndian>()?;
        let center_z = reader.read_f32::<LittleEndian>()?;
        let radius = reader.read_f32::<LittleEndian>()?;

        let mut vertex_chunk_list: Vec<VertexChunk> = Vec::new();
        if vlist_ptr != 0 {
            reader.seek(SeekFrom::Start(vlist_ptr))?;
            while let Some(chunk) = VertexChunk::from_buf(reader)? {
                vertex_chunk_list.push(chunk);
            }
        }

        let mut poly_bytes = Vec::new();
        let mut poly_chunk_list: Vec<PolyChunk> = Vec::new();
        if plist_ptr != 0 {
            reader.seek(SeekFrom::Start(plist_ptr))?;
            while let Some((vec, chunk)) = PolyChunk::from_buf(reader)? {
                poly_bytes.extend_from_slice(vec.as_slice());
                poly_chunk_list.push(chunk);
            }
        }

        let model = ChunkModel {
            vertex_list: vertex_chunk_list,
            poly_list: Some(poly_chunk_list),
            poly_bytes,
            _center: Point3 {
                x: center_x,
                y: center_y,
                z: center_z,
            },
            _r: radius,
        };

        Ok(model)
    }
}
