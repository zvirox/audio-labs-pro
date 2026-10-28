//! Typed identifiers for projects, sources, tracks and clips.
//!
//! UUID-backed newtypes keep identifiers from being mixed up at call sites while
//! remaining stable across saves and language boundaries.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generate a new random identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Build a deterministic identifier from a 128-bit value (tests, fixtures).
            #[must_use]
            pub fn from_u128(value: u128) -> Self {
                Self(Uuid::from_u128(value))
            }

            /// Borrow the underlying UUID.
            #[must_use]
            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({})", stringify!($name), self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }
    };
}

id_type! {
    /// Identifies a project.
    ProjectId
}

id_type! {
    /// Identifies an imported media source inside a project.
    SourceId
}

id_type! {
    /// Identifies a track.
    TrackId
}

id_type! {
    /// Identifies a clip.
    ClipId
}

id_type! {
    /// Identifies a timeline marker.
    MarkerId
}

id_type! {
    /// Identifies a timeline region.
    RegionId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_ids_round_trip_through_text() {
        let id = TrackId::from_u128(0x1234);
        assert_eq!(id.as_uuid(), TrackId::from_u128(0x1234).as_uuid());
        let json = serde_json::to_string(&id).expect("serialize");
        let back: TrackId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn new_ids_are_unique() {
        assert_ne!(SourceId::new(), SourceId::new());
    }
}
