// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Miscellaneous functions that save a lot of time writing boiler plate.

/// Creates an extra type safe u64 integer.
macro_rules! create_strong_type {
    ($name: ident) => {
        /// A type safe pointer to an object in a file.
        #[derive(Copy, Clone, Debug)]
        pub struct $name(u64);

        impl std::ops::Deref for $name {
            type Target = u64;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&other.0)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.eq(&other.0)
            }
        }

        impl std::hash::Hash for $name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }
    };
}
