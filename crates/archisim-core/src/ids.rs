//! Element identifiers.
//!
//! Every element has two identities:
//! - a small typed id ([`LevelId`], [`WallId`], …) used inside the document;
//! - an IFC [`GlobalId`] (22 characters) that never changes once created, so
//!   that exported models can be federated and compared between versions.

use std::fmt;

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    };
}

typed_id!(
    /// Identifies a [`crate::Level`].
    LevelId
);
typed_id!(
    /// Identifies a [`crate::Material`].
    MaterialId
);
typed_id!(
    /// Identifies a [`crate::WallType`].
    WallTypeId
);
typed_id!(
    /// Identifies a [`crate::Wall`].
    WallId
);
typed_id!(
    /// Identifies a [`crate::SlabType`].
    SlabTypeId
);
typed_id!(
    /// Identifies a [`crate::Slab`].
    SlabId
);
typed_id!(
    /// Identifies an [`crate::OpeningType`] (a door or window type).
    OpeningTypeId
);
typed_id!(
    /// Identifies an [`crate::Opening`] (a door or window in a wall).
    OpeningId
);

typed_id!(
    /// Identifies a [`crate::Family`] loaded in the project.
    FamilyId
);

typed_id!(
    /// Identifies a [`crate::Dimension`] (a linear or chained dimension).
    DimensionId
);
typed_id!(
    /// Identifies an [`crate::ElevationMark`] (a level mark in plan).
    ElevationMarkId
);

typed_id!(
    /// Identifies a [`crate::DrawLayer`] (a drawing layer, as in AutoCAD).
    DrawLayerId
);
typed_id!(
    /// Identifies a [`crate::Draft`] (a 2D drafting element: line, arc,
    /// text, hatch…).
    DraftId
);

typed_id!(
    /// Identifies a [`crate::Sheet`] (a drawing sheet, "plano").
    SheetId
);
typed_id!(
    /// Identifies a [`crate::Viewport`] (a view placed on a sheet).
    ViewportId
);

/// Alphabet used by IFC to compress a 128-bit GUID into 22 characters.
const IFC_GUID_ALPHABET: &[u8; 64] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

/// An IFC `IfcGloballyUniqueId`: a 128-bit value encoded in 22 characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalId(String);

impl GlobalId {
    /// Creates a new random identifier.
    pub fn new() -> Self {
        Self::from_u128(uuid::Uuid::new_v4().as_u128())
    }

    /// Encodes a 128-bit value: 2 bits in the first character, then 21
    /// characters of 6 bits each.
    pub fn from_u128(value: u128) -> Self {
        let mut s = String::with_capacity(22);
        s.push(IFC_GUID_ALPHABET[((value >> 126) & 0x3) as usize] as char);
        for i in (0..21).rev() {
            s.push(IFC_GUID_ALPHABET[((value >> (6 * i)) & 0x3f) as usize] as char);
        }
        Self(s)
    }

    /// Decodes back into the 128-bit value. Returns `None` if the text is
    /// not a valid IFC GUID.
    pub fn to_u128(&self) -> Option<u128> {
        let bytes = self.0.as_bytes();
        if bytes.len() != 22 {
            return None;
        }
        let mut value: u128 = 0;
        for (i, b) in bytes.iter().enumerate() {
            let digit = IFC_GUID_ALPHABET.iter().position(|c| c == b)? as u128;
            if i == 0 && digit > 3 {
                return None;
            }
            value = (value << if i == 0 { 0 } else { 6 }) | digit;
        }
        Some(value)
    }

    /// Parses an existing 22-character IFC GUID.
    pub fn parse(text: &str) -> Option<Self> {
        let id = Self(text.to_string());
        id.to_u128().map(|_| id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for GlobalId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GlobalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_is_22_chars_and_round_trips() {
        for _ in 0..200 {
            let g = GlobalId::new();
            assert_eq!(g.as_str().len(), 22);
            let v = g.to_u128().unwrap();
            assert_eq!(GlobalId::from_u128(v), g);
        }
    }

    #[test]
    fn guid_extremes() {
        assert_eq!(GlobalId::from_u128(0).as_str(), "0000000000000000000000");
        assert_eq!(
            GlobalId::from_u128(u128::MAX).as_str(),
            "3$$$$$$$$$$$$$$$$$$$$$"
        );
        assert!(GlobalId::parse("4$$$$$$$$$$$$$$$$$$$$$").is_none());
        assert!(GlobalId::parse("short").is_none());
    }
}
