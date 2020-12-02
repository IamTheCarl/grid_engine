// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Long term storage of the world on the local disk.

use anyhow::{Context, Result};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use fs::File;
use serde::{
    de::{self, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::{SerializeStruct, Serializer},
    Deserialize, Serialize,
};
use std::{
    fmt, fs,
    io::{BufReader, BufWriter, Cursor, Read, Write},
    path::{Path, PathBuf},
};

/// The number of bits in a block address that are specific to the block, and not part of the the chunk's address.
pub const BLOCK_ADDRESS_BITS: usize = 5;

/// The diameter of a chunk in blocks.
pub const CHUNK_DIAMETER: usize = 1 << BLOCK_ADDRESS_BITS;

// A chunk is 16x16x16 blocks in size, and a block consists of two bytes.
// That makes the chunk 8Kb in length.
const CHUNK_LENGTH: usize = CHUNK_DIAMETER * CHUNK_DIAMETER * CHUNK_DIAMETER * 2;

create_strong_type!(ChunkKey);

/// The raw data for a chunk.
pub struct ChunkData {
    storage: [u16; CHUNK_LENGTH],
    x: i16,
    y: i16,
    z: i16,
}

// We have to manually implement the serialization interfaces because we can't
// serialize our box safely.
impl Serialize for ChunkData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Chunk", 6)?;
        s.serialize_field("x", &self.x)?;
        s.serialize_field("y", &self.x)?;
        s.serialize_field("z", &self.x)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for Box<ChunkData> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            X,
            Y,
            Z,
        };

        struct ChunkVistor;

        impl<'de> Visitor<'de> for ChunkVistor {
            type Value = Box<ChunkData>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ChunkData")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Box<ChunkData>, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let x = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let y = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                let z = seq.next_element()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(ChunkData::create(x, y, z))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Box<ChunkData>, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut x = None;
                let mut y = None;
                let mut z = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::X => {
                            if x.is_some() {
                                return Err(de::Error::duplicate_field("x"));
                            }
                            x = Some(map.next_value()?);
                        }
                        Field::Y => {
                            if y.is_some() {
                                return Err(de::Error::duplicate_field("y"));
                            }
                            y = Some(map.next_value()?);
                        }
                        Field::Z => {
                            if z.is_some() {
                                return Err(de::Error::duplicate_field("z"));
                            }
                            z = Some(map.next_value()?);
                        }
                    }
                }
                let x = x.ok_or_else(|| de::Error::missing_field("x"))?;
                let y = y.ok_or_else(|| de::Error::missing_field("y"))?;
                let z = z.ok_or_else(|| de::Error::missing_field("z"))?;
                Ok(ChunkData::create(x, y, z))
            }
        }

        const FIELDS: &'static [&'static str] = &["x", "y", "z"];
        deserializer.deserialize_struct("Duration", FIELDS, ChunkVistor)
    }
}

impl ChunkData {
    /// Creates a chunk at the specified index.
    pub fn create(x: i16, y: i16, z: i16) -> Box<ChunkData> {
        // Due to a bug in Rust itself, we have to manually build this box.
        use std::{
            alloc::{alloc, Layout},
            ptr,
        };
        let mut chunk = unsafe {
            let pointer = alloc(Layout::new::<ChunkData>()) as *mut ChunkData;

            // We want the array to be filled with zeros, but we might as well clear
            // everything else while we're at it. Notice we use a 1. The
            // function write_bytes is a lot smarter than memset. It figures out that 1
            // means one whole struct.
            ptr::write_bytes(pointer, 0, 1);
            Box::from_raw(pointer)
        };

        chunk.x = x;
        chunk.y = y;
        chunk.z = z;

        chunk
    }

    /// Gets the index of this chunk.
    pub fn get_index(&self) -> (i16, i16, i16) {
        (self.x, self.y, self.z)
    }

    /// Provides the block data for this chunk.
    pub fn get_data(&self) -> &[u16] {
        &self.storage
    }

    /// Provides the block data for this chunk.
    pub fn get_data_mut(&mut self) -> &mut [u16] {
        &mut self.storage
    }
}

/// A struct that will store and fetch chunks. It will create new chunks if the
/// chunk does not exist in the file, but it will not fill the chunk with
/// content.
pub struct ChunkDiskStorage {
    root_folder: PathBuf,
    compression_level: Compression,
}

// Want to keep this thread safe.
static_assertions::assert_impl_all!(ChunkDiskStorage: Send, Sync);

impl ChunkDiskStorage {
    /// Provide a file handles for both the index file and the chunk file and
    /// this will be able to load and store terrain chunk data in them. Note
    /// that if the index file is uninitialized, this will go through the
    /// process of initializing them.
    pub fn initialize(root_folder: &Path, compression_level: u8) -> ChunkDiskStorage {
        ChunkDiskStorage {
            root_folder: PathBuf::from(root_folder),
            compression_level: Compression::new(compression_level as u32),
        }
    }

    /// Will get a single chunk's data at the specified chunk coordinates.
    /// Search time is filesystem dependent. If the chunk does not exist in
    /// the file, None will be returned.
    pub fn get_chunk(&self, x: i16, y: i16, z: i16) -> Result<Option<Box<ChunkData>>> {
        let mut chunk = ChunkData::create(x, y, z);

        if self.load_chunk(&mut chunk)? {
            Ok(Some(chunk))
        } else {
            Ok(None)
        }
    }

    /// Will load a chunk's terrain content. Search and fetch time is filesystem
    /// dependent. If the chunk does not exist, false will be returned.
    /// Otherwise, true is returned.
    pub fn load_chunk(&self, chunk: &mut ChunkData) -> Result<bool> {
        let path = self.create_chunk_path(chunk.x, chunk.y, chunk.z);

        if path.exists() {
            let file = File::open(path)?;
            let mut file = BufReader::new(file);
            let mut data = Vec::new();
            file.read_to_end(&mut data).context("Error while reading chunk file.")?;
            let data = Cursor::new(data);
            let mut zip = DeflateDecoder::new(data);
            {
                // We need to view this as bytes. Don't worry about the endian. We'll fix that
                // in a moment.
                let block_data = unsafe { std::mem::transmute::<&mut [u16], &mut [u8]>(chunk.get_data_mut()) };
                zip.read_exact(block_data).context("Failed to read bytes into chunk.")?;
            }

            // If we are a big endian machine, we have to flip all those bytes to our big
            // endian format.
            #[cfg(target_endian = "big")]
            {
                for block in chunk.get_data_mut() {
                    *block = u16::from_le_bytes(block.to_ne_bytes());
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Save the bytes of a chunk to a file.
    pub fn save_chunk(&self, chunk: &ChunkData) -> Result<()> {
        let path = self.create_chunk_path(chunk.x, chunk.y, chunk.z);
        if path.exists() {
            // We are going to make a backup of the old version of this file.
            let mut backup_path = path.clone();
            backup_path.set_extension(".backup");
            let backup_path = backup_path; // I just like to toss out mutability whenever I can.

            if backup_path.exists() {
                // Delete the old backup if it already exists.
                fs::remove_file(&backup_path)?;
            }

            // Move the old version into the backup.
            fs::rename(&path, backup_path)?;
        }

        let file = File::create(path)?;
        let mut file = BufWriter::new(file); // Makes writing small bits of data a little more efficient.
        let mut storage = Vec::new();
        storage.reserve(CHUNK_LENGTH);
        let mut compressor = DeflateEncoder::new(storage, self.compression_level);

        for block in chunk.get_data() {
            compressor.write(&block.to_le_bytes()).context("Error writing to compression buffer.")?;
        }

        let to_write = compressor.finish().context("Error compressing chunk")?;
        file.write_all(&to_write).context("Error writing chunk data to file.")?;

        Ok(())
    }

    /// If you want to be able to fetch a chunk from the index, you first need a
    /// chunk key. This will generate it from a chunk index.
    fn create_chunk_key(x: i16, y: i16, z: i16) -> ChunkKey {
        // We group bits of the three axis together so that the more significant bits
        // are on the left and the less significant are on the right. This
        // improves our chances of physically close chunks are close in the binary tree,
        // improving our iteration speed when requesting a range.
        fn spread_bits(input: i16) -> u64 {
            let mut input = input as u64 & 0x000000000000FFFF;
            let magic_numbers = [
                (32, 0x00FF00000000FFFF),
                (16, 0x00FF0000FF0000FF),
                (8, 0xF00F00F00F00F00F),
                (4, 0x30C30C30C30C30C3),
                (2, 0x9249249249249249),
            ];

            // TODO should be loop unrolling on its own but I should check this.
            for (shift, mask) in &magic_numbers {
                input = (input | (input << shift)) & mask;
            }

            input
        }

        let x = spread_bits(x);
        let y = spread_bits(y);
        let z = spread_bits(z);

        // Return all of these spaced out versions of the keys ored together.
        ChunkKey((x << 2) | (y << 1) | z)
    }

    fn create_chunk_file_name(key: ChunkKey) -> String {
        format!("{:012X}", key.0)
    }

    fn create_chunk_path(&self, x: i16, y: i16, z: i16) -> PathBuf {
        let key = Self::create_chunk_key(x, y, z);

        self.root_folder.join(PathBuf::from(Self::create_chunk_file_name(key)))
    }
}

#[cfg(test)]
mod test_fileformate {

    use super::*;

    #[test]
    fn read_chunk_doesnt_exist() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ChunkDiskStorage::initialize(dir.path(), 9);
        assert!(storage.get_chunk(0, 0, 0).unwrap().is_none());
    }

    #[test]
    fn create_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ChunkDiskStorage::initialize(dir.path(), 9);
        let chunk = ChunkData::create(0, 0, 0);
        storage.save_chunk(&chunk).unwrap();

        assert!(storage.get_chunk(0, 0, 0).unwrap().is_some());
    }

    #[test]
    #[allow(overflowing_literals)] // Makes it so we can ignore the overflow when writing hexadecimal.
    fn generate_chunk_file_names() {
        // This just verifies that we are creating file names from chunk keys. It
        // doesn't go into the details of the keys.
        assert_eq!(
            ChunkDiskStorage::create_chunk_file_name(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0000)),
            "000000000000"
        );

        assert_eq!(
            ChunkDiskStorage::create_chunk_file_name(ChunkDiskStorage::create_chunk_key(0x8000, 0x8000, 0x8000)),
            "E00000000000"
        );

        assert_eq!(
            ChunkDiskStorage::create_chunk_file_name(ChunkDiskStorage::create_chunk_key(0x0800, 0x0800, 0x0800)),
            "000E00000000"
        );

        assert_eq!(
            ChunkDiskStorage::create_chunk_file_name(ChunkDiskStorage::create_chunk_key(0x0080, 0x0080, 0x0080)),
            "000000E00000"
        );

        assert_eq!(
            ChunkDiskStorage::create_chunk_file_name(ChunkDiskStorage::create_chunk_key(0x0008, 0x0008, 0x0008)),
            "000000000E00"
        );
    }

    #[test]
    #[allow(overflowing_literals)] // Makes it so we can ignore the overflow when writing hexadecimal.
    fn chunk_keys() {
        // We test that every bit maps correctly.

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0000), ChunkKey(0x0000000000000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x8000, 0x0000, 0x0000), ChunkKey(0x0000800000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x8000, 0x0000), ChunkKey(0x0000400000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x8000), ChunkKey(0x0000200000000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x4000, 0x0000, 0x0000), ChunkKey(0x0000100000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x4000, 0x0000), ChunkKey(0x0000080000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x4000), ChunkKey(0x0000040000000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x2000, 0x0000, 0x0000), ChunkKey(0x0000020000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x2000, 0x0000), ChunkKey(0x0000010000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x2000), ChunkKey(0x0000008000000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x1000, 0x0000, 0x0000), ChunkKey(0x0000004000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x1000, 0x0000), ChunkKey(0x0000002000000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x1000), ChunkKey(0x0000001000000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0800, 0x0000, 0x0000), ChunkKey(0x0000000800000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0800, 0x0000), ChunkKey(0x0000000400000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0800), ChunkKey(0x0000000200000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0400, 0x0000, 0x0000), ChunkKey(0x0000000100000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0400, 0x0000), ChunkKey(0x0000000080000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0400), ChunkKey(0x0000000040000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0200, 0x0000, 0x0000), ChunkKey(0x0000000020000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0200, 0x0000), ChunkKey(0x0000000010000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0200), ChunkKey(0x0000000008000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0100, 0x0000, 0x0000), ChunkKey(0x0000000004000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0100, 0x0000), ChunkKey(0x0000000002000000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0100), ChunkKey(0x0000000001000000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0080, 0x0000, 0x0000), ChunkKey(0x0000000000800000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0080, 0x0000), ChunkKey(0x0000000000400000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0080), ChunkKey(0x0000000000200000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0040, 0x0000, 0x0000), ChunkKey(0x0000000000100000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0040, 0x0000), ChunkKey(0x0000000000080000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0040), ChunkKey(0x0000000000040000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0020, 0x0000, 0x0000), ChunkKey(0x0000000000020000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0020, 0x0000), ChunkKey(0x0000000000010000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0020), ChunkKey(0x0000000000008000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0010, 0x0000, 0x0000), ChunkKey(0x0000000000004000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0010, 0x0000), ChunkKey(0x0000000000002000));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0010), ChunkKey(0x0000000000001000));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0008, 0x0000, 0x0000), ChunkKey(0x0000000000000800));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0008, 0x0000), ChunkKey(0x0000000000000400));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0008), ChunkKey(0x0000000000000200));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0004, 0x0000, 0x0000), ChunkKey(0x0000000000000100));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0004, 0x0000), ChunkKey(0x0000000000000080));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0004), ChunkKey(0x0000000000000040));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0002, 0x0000, 0x0000), ChunkKey(0x0000000000000020));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0002, 0x0000), ChunkKey(0x0000000000000010));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0002), ChunkKey(0x0000000000000008));

        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0001, 0x0000, 0x0000), ChunkKey(0x0000000000000004));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0001, 0x0000), ChunkKey(0x0000000000000002));
        assert_eq!(ChunkDiskStorage::create_chunk_key(0x0000, 0x0000, 0x0001), ChunkKey(0x0000000000000001));
    }
}
