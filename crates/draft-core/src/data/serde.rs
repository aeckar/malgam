use std::{collections::HashMap, fmt};

use ordered_float::NotNan;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
};

use crate::data::DataValue;

impl Serialize for DataValue {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            DataValue::Null => s.serialize_unit(),
            DataValue::Bool(b) => s.serialize_bool(*b),
            DataValue::Number(n) => s.serialize_f64(n.into_inner()),
            DataValue::String(st) => s.serialize_str(st),
            DataValue::List(items) => {
                let mut seq = s.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }

            // Objects are serialized as maps. The `tag` is emitted under the
            // reserved key `"$tag"` so it survives a round-trip. The key is
            // omitted entirely when the tag is empty (anonymous object).
            DataValue::Object { tag, map } => {
                let extra = if tag.is_empty() { 0 } else { 1 };
                let mut m = s.serialize_map(Some(map.len() + extra))?;
                if !tag.is_empty() {
                    m.serialize_entry("$tag", tag)?;
                }
                for (k, v) in map {
                    m.serialize_entry(k, v)?;
                }
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for DataValue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(DataValueVisitor)
    }
}

struct DataValueVisitor;

impl<'de> Visitor<'de> for DataValueVisitor {
    type Value = DataValue;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a DataValue (null, bool, number, string, list, or object)")
    }

    // ----------------------------------------- scalars -----------------------------------------

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(DataValue::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(DataValue::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        DataValue::deserialize(d)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(DataValue::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        self.visit_f64(v as f64)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        self.visit_f64(v as f64)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        NotNan::new(v)
            .map(DataValue::Number)
            .map_err(|_| de::Error::custom("NaN is not a valid DataValue::Number"))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(DataValue::String(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(DataValue::String(v))
    }

    // ------------------------------------- sequence -> List -------------------------------------

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(v) = seq.next_element::<DataValue>()? {
            items.push(v);
        }
        Ok(DataValue::List(items))
    }

    // -------------------------------------- map -> object --------------------------------------
    //
    // The reserved key `"$tag"` is lifted out to become the `tag` field;
    // everything else becomes a regular map entry.

    fn visit_map<A: MapAccess<'de>>(self, mut map_access: A) -> Result<Self::Value, A::Error> {
        let mut tag = String::new();
        let mut map = HashMap::new();
        while let Some(k) = map_access.next_key::<String>()? {
            if k == "$tag" {
                tag = map_access.next_value::<String>()?;
            } else {
                let v = map_access.next_value::<DataValue>()?;
                map.insert(k, v);
            }
        }
        Ok(DataValue::Object { tag, map })
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*; // convenient round-trip target

    fn num(v: f64) -> DataValue {
        DataValue::Number(NotNan::new(v).unwrap())
    }

    // ---------------------------------------- serialize ----------------------------------------

    #[test]
    fn serialize_null() {
        assert_eq!(serde_json::to_string(&DataValue::Null).unwrap(), "null");
    }

    #[test]
    fn serialize_bool() {
        assert_eq!(
            serde_json::to_string(&DataValue::Bool(true)).unwrap(),
            "true"
        );
        assert_eq!(
            serde_json::to_string(&DataValue::Bool(false)).unwrap(),
            "false"
        );
    }

    #[test]
    fn serialize_number() {
        assert_eq!(serde_json::to_string(&num(3.14)).unwrap(), "3.14");
    }

    #[test]
    fn serialize_string() {
        let v = DataValue::String("hello".into());
        assert_eq!(serde_json::to_string(&v).unwrap(), r#""hello""#);
    }

    #[test]
    fn serialize_list() {
        let v = DataValue::List(vec![num(1.0), DataValue::Bool(false), DataValue::Null]);
        assert_eq!(serde_json::to_string(&v).unwrap(), "[1.0,false,null]");
    }

    #[test]
    fn serialize_object_with_tag() {
        let v = DataValue::Object {
            tag: "point".into(),
            map: [("x".into(), num(1.0)), ("y".into(), num(2.0))]
                .into_iter()
                .collect(),
        };
        let s = serde_json::to_string(&v).unwrap();
        assert!(s.contains(r#""$tag":"point""#));
        assert!(s.contains(r#""x":1.0"#));
        assert!(s.contains(r#""y":2.0"#));
    }

    #[test]
    fn serialize_object_empty_tag_omitted() {
        let v = DataValue::Object {
            tag: "".into(),
            map: [("k".into(), DataValue::Bool(true))].into_iter().collect(),
        };
        let s = serde_json::to_string(&v).unwrap();
        assert!(!s.contains("$tag"));
    }

    // --------------------------------------- deserialize ---------------------------------------

    #[test]
    fn deserialize_null() {
        assert_eq!(
            serde_json::from_str::<DataValue>("null").unwrap(),
            DataValue::Null
        );
    }

    #[test]
    fn deserialize_bool() {
        assert_eq!(
            serde_json::from_str::<DataValue>("true").unwrap(),
            DataValue::Bool(true)
        );
    }

    #[test]
    fn deserialize_number() {
        assert_eq!(
            serde_json::from_str::<DataValue>("42.0").unwrap(),
            num(42.0)
        );
    }

    #[test]
    fn deserialize_string() {
        assert_eq!(
            serde_json::from_str::<DataValue>(r#""world""#).unwrap(),
            DataValue::String("world".into())
        );
    }

    #[test]
    fn deserialize_list() {
        let v: DataValue = serde_json::from_str("[1.0, true, null]").unwrap();
        assert_eq!(
            v,
            DataValue::List(vec![num(1.0), DataValue::Bool(true), DataValue::Null])
        );
    }

    #[test]
    fn roundtrip_object() {
        let original = DataValue::Object {
            tag: "rect".into(),
            map: [("w".into(), num(100.0)), ("h".into(), num(50.0))]
                .into_iter()
                .collect(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let recovered: DataValue = serde_json::from_str(&json).unwrap();
        assert_eq!(original, recovered);
    }
}
