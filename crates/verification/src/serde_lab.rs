//! Lecture et écriture JSON des couleurs Lab de `colorimetrie`, qui ne
//! dépend pas de serde : `{"l": …, "a": …, "b": …}`.

use colorimetrie::Lab;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct LabJson {
    l: f64,
    a: f64,
    b: f64,
}

impl From<Lab> for LabJson {
    fn from(c: Lab) -> Self {
        LabJson {
            l: c.l,
            a: c.a,
            b: c.b,
        }
    }
}

impl From<LabJson> for Lab {
    fn from(c: LabJson) -> Self {
        Lab {
            l: c.l,
            a: c.a,
            b: c.b,
        }
    }
}

pub mod un {
    use super::*;

    pub fn serialize<S: Serializer>(c: &Lab, s: S) -> Result<S::Ok, S::Error> {
        LabJson::from(*c).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Lab, D::Error> {
        LabJson::deserialize(d).map(Lab::from)
    }
}

pub mod option {
    use super::*;

    pub fn serialize<S: Serializer>(c: &Option<Lab>, s: S) -> Result<S::Ok, S::Error> {
        c.map(LabJson::from).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Lab>, D::Error> {
        Option::<LabJson>::deserialize(d).map(|c| c.map(Lab::from))
    }
}

pub mod liste {
    use super::*;

    pub fn serialize<S: Serializer>(c: &[Lab], s: S) -> Result<S::Ok, S::Error> {
        c.iter()
            .copied()
            .map(LabJson::from)
            .collect::<Vec<_>>()
            .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Lab>, D::Error> {
        Vec::<LabJson>::deserialize(d).map(|c| c.into_iter().map(Lab::from).collect())
    }
}

pub mod option_liste {
    use super::*;

    pub fn serialize<S: Serializer>(c: &Option<Vec<Lab>>, s: S) -> Result<S::Ok, S::Error> {
        c.as_ref()
            .map(|v| v.iter().copied().map(LabJson::from).collect::<Vec<_>>())
            .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<Lab>>, D::Error> {
        Option::<Vec<LabJson>>::deserialize(d)
            .map(|c| c.map(|v| v.into_iter().map(Lab::from).collect()))
    }
}
