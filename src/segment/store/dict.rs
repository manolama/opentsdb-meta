/*
 *
 *  * This file is part of OpenTSDB.
 *  * Copyright (C) 2021  Yahoo.
 *  *
 *  * Licensed under the Apache License, Version 2.0 (the "License");
 *  * you may not use this file except in compliance with the License.
 *  * You may obtain a copy of the License at
 *  *
 *  *   http://www.apache.org/licenses/LICENSE-2.0
 *  *
 *  * Unless required by applicable law or agreed to in writing, software
 *  * distributed under the License is distributed on an "AS IS" BASIS,
 *  * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  * See the License for the specific language governing permissions and
 *  * limitations under the License.
 *
 */

use byteorder::{NativeEndian, WriteBytesExt};

use std::{collections::HashMap, io::Read, io::Write, rc::Rc};

use crate::segment::persistence::Builder;
use crate::segment::persistence::Loader;
use crate::segment::segment_reader::SegmentReader;
use crate::utils::myst_error::MystError;
use crate::utils::myst_error::Result;
use std::io::{BufReader, Seek, SeekFrom};
use std::sync::Arc;

/// Stores the dictionary data structure of the segment.
/// Dictionary is essentially a HashMap with an integer key and
/// a String as value.
#[derive(Debug, Default)]
pub struct Dict {
    pub dict: HashMap<u32, Rc<String>>,
}

impl<W: Write> Builder<W> for Dict {
    fn build(self, buf: &mut W, offset: &mut u32) -> Result<Option<Self>> {
        let mut serialized = Vec::new();
        let mut len = 0;
        len += 4;
        serialized.write_u32::<NativeEndian>(self.dict.len() as u32)?;

        for (k, v) in self.dict.iter() {
            len += 4;
            serialized.write_u32::<NativeEndian>(*k)?;
            len += 4;
            serialized.write_u32::<NativeEndian>(v.len() as u32)?;
            len += v.as_bytes().len();
            serialized.extend(v.as_bytes());
        }

        *offset += serialized.len() as u32;
        buf.write_all(&serialized)?;
        Ok(Some(self))
    }
}

impl<R: Read + Seek> Loader<R, Dict> for Dict {
    fn load(mut self, buf: &mut R, offset: &u32) -> Result<Option<Dict>> {
        buf.seek(SeekFrom::Start(*offset as u64))?;
        let dict_map = SegmentReader::get_dict_from_reader(buf)?;
        match Arc::try_unwrap(dict_map) {
            Ok(dict) => {
                let mut rc_dict = HashMap::new();
                for (k, v) in dict {
                    rc_dict.insert(k, Rc::new(v));
                }
                return Ok(Some(Dict { dict: rc_dict }));
            }
            Err(_) => return Err(MystError::new_query_error("cannot unwrap to dict")),
        }
    }
}

impl Dict {
    pub fn new() -> Self {
        Self {
            dict: HashMap::default(),
        }
    }
}
