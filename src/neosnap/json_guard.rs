use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::{collections::HashSet, fmt};

struct Guard<'a> {
    depth: usize,
    nodes: &'a mut usize,
}
impl<'de> DeserializeSeed<'de> for Guard<'_> {
    type Value = ();
    fn deserialize<D: de::Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        *self.nodes += 1;
        if self.depth > 16 || *self.nodes > 4_100_000 {
            return Err(de::Error::custom("JSON limit"));
        }
        d.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Guard<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bounded JSON")
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: de::Error>(self, n: f64) -> Result<(), E> {
        if n.is_finite() {
            Ok(())
        } else {
            Err(E::custom("Invalid number"))
        }
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        let mut count = 0;
        while a
            .next_element_seed(Guard {
                depth: self.depth + 1,
                nodes: self.nodes,
            })?
            .is_some()
        {
            count += 1;
            if count > 100_000 {
                return Err(de::Error::custom("Array limit"));
            }
        }
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<(), A::Error> {
        let mut keys = HashSet::new();
        while let Some(key) = a.next_key::<String>()? {
            if key.len() > 128 || keys.len() >= 64 || !keys.insert(key) {
                return Err(de::Error::custom("Duplicate key or map limit"));
            }
            a.next_value_seed(Guard {
                depth: self.depth + 1,
                nodes: self.nodes,
            })?;
        }
        Ok(())
    }
}
pub(super) fn check(bytes: &[u8]) -> Result<(), serde_json::Error> {
    let mut d = serde_json::Deserializer::from_slice(bytes);
    Guard {
        depth: 0,
        nodes: &mut 0,
    }
    .deserialize(&mut d)?;
    d.end()
}
