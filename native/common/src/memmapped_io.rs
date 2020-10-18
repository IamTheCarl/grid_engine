// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Helper tools for memory mapped file IO.
//!
//! The MMapped types permit reading and writing integers to and from memory mapped files in
//! a platform agnostic way. You see, they keep the endian of the files in mind. The files will
//! always have their data stored in little endian format, no matter what the system's endian is.
//! Little endian was chosen because it is the more common architecture this game is expected to run on.

macro_rules! implement_integer_type {
    ($struct_name: ident, $accessor_name: ident, $type: ty) => {
        /// An integer mapped to memory. When written to, it will always be stored in little endian format, no
        /// matter what the architecture natively supports. The intended use for this is accessing data in
        /// memory mapped files.
        pub struct $struct_name<'a> {
            bytes: &'a mut [u8; std::mem::size_of::<$type>()],
        }

        impl<'a> $struct_name<'a> {
            /// Construct a new instance of the MMapped referenced to the memory pointed to by bytes.
            pub fn new(bytes: &'a mut [u8; std::mem::size_of::<$type>()]) -> Self {
                Self { bytes }
            }

            /// Get an immutable accessor for this data.
            pub fn access_mut<'b>(&'b mut self) -> $accessor_name<'a, 'b> {
                $accessor_name::new(self)
            }

            /// Just read the value stored at that point in memory.
            pub fn read(&self) -> $type {
                <$type>::from_le_bytes(self.bytes.clone())
            }
        }

        /// An accessor to the MMapped integer. It keeps a native endian copy of the variable that can be quickly
        /// accessed and/or modified. Whenever flush() is called, or if the struct is dropped, the value will then
        /// be converted to little endian and stored in its source memory.
        pub struct $accessor_name<'a, 'b> {
            owner: &'a mut $struct_name<'b>,
            local_copy: $type,
        }

        impl<'a, 'b> std::fmt::Display for $accessor_name<'a, 'b> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.local_copy)
            }
        }

        impl<'a, 'b> std::fmt::Debug for $accessor_name<'a, 'b> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.local_copy)
            }
        }

        impl<'a, 'b> $accessor_name<'a, 'b> {
            fn new(owner: &'a mut $struct_name<'b>) -> Self {
                // We get a local copy for faster manipulation.
                let local_copy = <$type>::from_le_bytes(owner.bytes.clone());
                Self { owner, local_copy }
            }
        }

        impl<'a, 'b> std::ops::Deref for $accessor_name<'a, 'b> {
            type Target = $type;
            fn deref(&self) -> &Self::Target {
                &self.local_copy
            }
        }

        impl<'a, 'b> std::ops::DerefMut for $accessor_name<'a, 'b> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.local_copy
            }
        }

        impl<'a, 'b> std::ops::Drop for $accessor_name<'a, 'b> {
            fn drop(&mut self) {
                // When we drop, we write our value to our owner.
                *self.owner.bytes = self.local_copy.to_le_bytes();
            }
        }
    };
}

// We use a macro to do this because it saves me a lot of work.
implement_integer_type!(MMappedU16, MMappedU16Accessor, u16);
implement_integer_type!(MMappedI16, MMappedI16Accessor, i16);
implement_integer_type!(MMappedU32, MMappedU32Accessor, u32);
implement_integer_type!(MMappedI32, MMappedI32Accessor, i32);
implement_integer_type!(MMappedU64, MMappedU64Accessor, u64);
implement_integer_type!(MMappedI64, MMappedI64Accessor, i64);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn just_read() {
        let mut data = [0x01u8, 0x02u8];
        let reference = MMappedU16::new(&mut data);
        let read = reference.read();
        assert_eq!(read, 0x0201u16);
    }

    #[test]
    fn read_write() {
        let mut data = [0x01u8, 0x02u8];
        let mut reference = MMappedU16::new(&mut data);
        let read = reference.read();
        assert_eq!(read, 0x0201u16);

        let mut access = reference.access_mut();
        assert_eq!(access, 0x0201u16);
        *access = 0x0102u16;
        assert_eq!(access, 0x0102u16);
        let read = reference.read();
        assert_eq!(read, 0x0102u16);

        assert_eq!(data, [0x02u8, 0x01u8]);
    }
}
