use serde_json::Value as JsonValue;

/// Represents an AMF value (shared between AMF0 and AMF3)
#[derive(Debug, Clone)]
pub enum AmfValue {
    Undefined,
    Null,
    Bool(bool),
    Integer(i32),
    Double(f64),
    String(String),
    Date(f64, i16),
    Array {
        dense: Vec<AmfValue>,
        assoc: Vec<(String, AmfValue)>,
    },
    Object {
        class: String,
        sealed: Vec<(String, AmfValue)>,
        dynamic: Vec<(String, AmfValue)>,
    },
}

/// Parser state holding reference tables and cursor position
pub struct AmfReader<'a> {
    data: &'a [u8],
    pos: usize,
    string_table: Vec<String>,
    object_table: Vec<()>,
    trait_table: Vec<TraitInfo>,
}

#[derive(Debug, Clone)]
struct TraitInfo {
    class_name: String,
    is_dynamic: bool,
    sealed_names: Vec<String>,
}

impl<'a> AmfReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            string_table: Vec::new(),
            object_table: Vec::new(),
            trait_table: Vec::new(),
        }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err(format!("unexpected EOF at position {}", self.pos));
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u16_be(&mut self) -> Result<u16, String> {
        if self.pos + 2 > self.data.len() {
            return Err(format!("unexpected EOF reading u16 at {}", self.pos));
        }
        let val = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    fn read_u32_be(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.data.len() {
            return Err(format!("unexpected EOF reading u32 at {}", self.pos));
        }
        let val = u32::from_be_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(val)
    }

    fn read_f64_be(&mut self) -> Result<f64, String> {
        if self.pos + 8 > self.data.len() {
            return Err(format!("unexpected EOF reading f64 at {}", self.pos));
        }
        let val = f64::from_be_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(val)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.pos + len > self.data.len() {
            return Err(format!(
                "unexpected EOF reading {} bytes at {}",
                len, self.pos
            ));
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    /// Read a U29 variable-length integer.
    pub fn read_u29(&mut self) -> Result<u32, String> {
        let mut result: u32 = 0;
        for i in 0..4u8 {
            let b = self.read_u8()? as u32;
            if i < 3 {
                result = (result << 7) | (b & 0x7F);
                if b & 0x80 == 0 {
                    return Ok(result);
                }
            } else {
                result = (result << 8) | b;
            }
        }
        Ok(result)
    }

    /// Read an AMF3 string (U29-prefixed, with reference table).
    pub fn read_string(&mut self) -> Result<String, String> {
        let ref_or_len = self.read_u29()?;
        if ref_or_len & 1 == 0 {
            let idx = (ref_or_len >> 1) as usize;
            self.string_table
                .get(idx)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "invalid string ref {} (table size {})",
                        idx,
                        self.string_table.len()
                    )
                })
        } else {
            let len = (ref_or_len >> 1) as usize;
            if len == 0 {
                return Ok(String::new());
            }
            let bytes = self.read_bytes(len)?;
            let s = String::from_utf8(bytes.to_vec())
                .map_err(|e| format!("invalid UTF-8 in string at {}: {e}", self.pos - len))?;
            self.string_table.push(s.clone());
            Ok(s)
        }
    }

    /// Read a single AMF3 typed value.
    pub fn read_value(&mut self) -> Result<AmfValue, String> {
        let marker = self.read_u8()?;
        match marker {
            0x00 => Ok(AmfValue::Undefined),
            0x01 => Ok(AmfValue::Null),
            0x02 => Ok(AmfValue::Bool(false)),
            0x03 => Ok(AmfValue::Bool(true)),
            0x04 => {
                let raw = self.read_u29()?;
                let val = if raw >= (1 << 28) {
                    (raw as i32) - (1 << 29)
                } else {
                    raw as i32
                };
                Ok(AmfValue::Integer(val))
            }
            0x05 => {
                let val = self.read_f64_be()?;
                Ok(AmfValue::Double(val))
            }
            0x06 => {
                let s = self.read_string()?;
                Ok(AmfValue::String(s))
            }
            0x08 => {
                // AMF3 Date: U29 ref/inline, then f64 millis
                let ref_or_inline = self.read_u29()?;
                if ref_or_inline & 1 == 0 {
                    return Err("date reference not supported".to_string());
                }
                self.object_table.push(());
                let ms = self.read_f64_be()?;
                Ok(AmfValue::Date(ms, 0))
            }
            0x09 => self.read_array(),
            0x0A => self.read_object(),
            other => Err(format!(
                "unsupported AMF3 marker 0x{:02x} at position {}",
                other,
                self.pos - 1
            )),
        }
    }

    fn read_array(&mut self) -> Result<AmfValue, String> {
        let ref_or_count = self.read_u29()?;
        if ref_or_count & 1 == 0 {
            return Err(format!(
                "array reference not supported (ref {})",
                ref_or_count >> 1
            ));
        }
        let dense_count = (ref_or_count >> 1) as usize;
        self.object_table.push(());

        // Associative portion: key-value pairs until empty string
        let mut assoc = Vec::new();
        loop {
            let key = self.read_string()?;
            if key.is_empty() {
                break;
            }
            let val = self.read_value()?;
            assoc.push((key, val));
        }

        // Dense portion
        let mut dense = Vec::with_capacity(dense_count);
        for _ in 0..dense_count {
            dense.push(self.read_value()?);
        }

        Ok(AmfValue::Array { dense, assoc })
    }

    fn read_object(&mut self) -> Result<AmfValue, String> {
        let ref_or_traits = self.read_u29()?;
        if ref_or_traits & 1 == 0 {
            return Err(format!(
                "object reference not supported (ref {})",
                ref_or_traits >> 1
            ));
        }

        self.object_table.push(());

        let trait_info = if ref_or_traits & 0x02 == 0 {
            // Trait reference
            let trait_idx = (ref_or_traits >> 2) as usize;
            self.trait_table
                .get(trait_idx)
                .cloned()
                .ok_or_else(|| format!("invalid trait ref {}", trait_idx))?
        } else {
            // Inline traits
            // bit 2 = externalizable (not supported)
            if ref_or_traits & 0x04 != 0 {
                let class_name = self.read_string()?;
                return Err(format!(
                    "externalizable object '{}' not supported",
                    class_name
                ));
            }
            let is_dynamic = (ref_or_traits & 0x08) != 0;
            let sealed_count = (ref_or_traits >> 4) as usize;
            let class_name = self.read_string()?;

            let mut sealed_names = Vec::with_capacity(sealed_count);
            for _ in 0..sealed_count {
                sealed_names.push(self.read_string()?);
            }

            let info = TraitInfo {
                class_name,
                is_dynamic,
                sealed_names,
            };
            self.trait_table.push(info.clone());
            info
        };

        // Read sealed property values
        let mut sealed = Vec::with_capacity(trait_info.sealed_names.len());
        for name in &trait_info.sealed_names {
            let val = self.read_value()?;
            sealed.push((name.clone(), val));
        }

        // Read dynamic properties
        let mut dynamic = Vec::new();
        if trait_info.is_dynamic {
            loop {
                let key = self.read_string()?;
                if key.is_empty() {
                    break;
                }
                let val = self.read_value()?;
                dynamic.push((key, val));
            }
        }

        Ok(AmfValue::Object {
            class: trait_info.class_name,
            sealed,
            dynamic,
        })
    }
}

// --- JSON conversion ---

impl AmfValue {
    pub fn to_json(&self) -> JsonValue {
        match self {
            AmfValue::Undefined => JsonValue::Null,
            AmfValue::Null => JsonValue::Null,
            AmfValue::Bool(b) => JsonValue::Bool(*b),
            AmfValue::Integer(i) => JsonValue::Number((*i).into()),
            AmfValue::Double(d) => serde_json::Number::from_f64(*d)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            AmfValue::String(s) => JsonValue::String(s.clone()),
            AmfValue::Date(ms, tz) => {
                let mut map = serde_json::Map::new();
                map.insert("__type__".to_string(), JsonValue::String("Date".to_string()));
                map.insert(
                    "ms".to_string(),
                    serde_json::Number::from_f64(*ms)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null),
                );
                map.insert("tz".to_string(), JsonValue::Number((*tz as i64).into()));
                JsonValue::Object(map)
            }
            AmfValue::Array { dense, assoc } => {
                if assoc.is_empty() {
                    JsonValue::Array(dense.iter().map(|v| v.to_json()).collect())
                } else {
                    let mut map = serde_json::Map::new();
                    for (k, v) in assoc {
                        map.insert(k.clone(), v.to_json());
                    }
                    if !dense.is_empty() {
                        map.insert(
                            "__dense__".to_string(),
                            JsonValue::Array(dense.iter().map(|v| v.to_json()).collect()),
                        );
                    }
                    JsonValue::Object(map)
                }
            }
            AmfValue::Object { class, sealed, dynamic } => {
                let mut map = serde_json::Map::new();
                if !class.is_empty() {
                    map.insert("__class__".to_string(), JsonValue::String(class.clone()));
                }
                for (k, v) in sealed {
                    map.insert(k.clone(), v.to_json());
                }
                for (k, v) in dynamic {
                    map.insert(k.clone(), v.to_json());
                }
                JsonValue::Object(map)
            }
        }
    }

    pub fn from_json(val: &JsonValue) -> Self {
        match val {
            JsonValue::Null => AmfValue::Null,
            JsonValue::Bool(b) => AmfValue::Bool(*b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        AmfValue::Integer(i as i32)
                    } else {
                        AmfValue::Double(i as f64)
                    }
                } else {
                    AmfValue::Double(n.as_f64().unwrap_or(0.0))
                }
            }
            JsonValue::String(s) => AmfValue::String(s.clone()),
            JsonValue::Array(arr) => AmfValue::Array {
                dense: arr.iter().map(AmfValue::from_json).collect(),
                assoc: Vec::new(),
            },
            JsonValue::Object(map) => {
                // Date object
                if map.get("__type__").and_then(|v| v.as_str()) == Some("Date") {
                    let ms = map
                        .get("ms")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let tz = map
                        .get("tz")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i16;
                    return AmfValue::Date(ms, tz);
                }
                if map.contains_key("__class__") {
                    let class = map
                        .get("__class__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let dynamic: Vec<(String, AmfValue)> = map
                        .iter()
                        .filter(|(k, _)| k.as_str() != "__class__")
                        .map(|(k, v)| (k.clone(), AmfValue::from_json(v)))
                        .collect();
                    AmfValue::Object {
                        class,
                        sealed: Vec::new(),
                        dynamic,
                    }
                } else if map.contains_key("__dense__") {
                    let dense = map
                        .get("__dense__")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().map(AmfValue::from_json).collect())
                        .unwrap_or_default();
                    let assoc: Vec<(String, AmfValue)> = map
                        .iter()
                        .filter(|(k, _)| k.as_str() != "__dense__")
                        .map(|(k, v)| (k.clone(), AmfValue::from_json(v)))
                        .collect();
                    AmfValue::Array { dense, assoc }
                } else {
                    let dynamic: Vec<(String, AmfValue)> = map
                        .iter()
                        .map(|(k, v)| (k.clone(), AmfValue::from_json(v)))
                        .collect();
                    AmfValue::Object {
                        class: String::new(),
                        sealed: Vec::new(),
                        dynamic,
                    }
                }
            }
        }
    }
}

// --- AMF3 Writer ---

pub struct AmfWriter {
    pub buf: Vec<u8>,
    string_table: Vec<String>,
}

impl AmfWriter {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            string_table: Vec::new(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    fn write_u8(&mut self, val: u8) {
        self.buf.push(val);
    }

fn write_f64_be(&mut self, val: f64) {
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_u29(&mut self, mut val: u32) {
        val &= 0x1FFFFFFF;
        if val < 0x80 {
            self.write_u8(val as u8);
        } else if val < 0x4000 {
            self.write_u8(((val >> 7) | 0x80) as u8);
            self.write_u8((val & 0x7F) as u8);
        } else if val < 0x200000 {
            self.write_u8(((val >> 14) | 0x80) as u8);
            self.write_u8(((val >> 7) | 0x80) as u8);
            self.write_u8((val & 0x7F) as u8);
        } else {
            self.write_u8(((val >> 22) | 0x80) as u8);
            self.write_u8(((val >> 15) | 0x80) as u8);
            self.write_u8(((val >> 8) | 0x80) as u8);
            self.write_u8((val & 0xFF) as u8);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        if !s.is_empty() {
            if let Some(idx) = self.string_table.iter().position(|x| x == s) {
                self.write_u29((idx as u32) << 1);
                return;
            }
            self.string_table.push(s.to_string());
        }
        let bytes = s.as_bytes();
        self.write_u29(((bytes.len() as u32) << 1) | 1);
        self.buf.extend_from_slice(bytes);
    }

    pub fn write_value(&mut self, val: &AmfValue) {
        match val {
            AmfValue::Undefined => self.write_u8(0x00),
            AmfValue::Null => self.write_u8(0x01),
            AmfValue::Bool(false) => self.write_u8(0x02),
            AmfValue::Bool(true) => self.write_u8(0x03),
            AmfValue::Integer(i) => {
                self.write_u8(0x04);
                let raw = if *i < 0 {
                    ((*i as i64) + (1i64 << 29)) as u32
                } else {
                    *i as u32
                };
                self.write_u29(raw);
            }
            AmfValue::Double(d) => {
                self.write_u8(0x05);
                self.write_f64_be(*d);
            }
            AmfValue::String(s) => {
                self.write_u8(0x06);
                self.write_string(s);
            }
            AmfValue::Date(ms, tz) => {
                // AMF3 date: marker 0x08, U29 (inline flag), f64 millis
                // AMF3 dates don't carry timezone — store as inline + millis only
                self.write_u8(0x08);
                self.write_u29(1); // inline (low bit = 1)
                self.write_f64_be(*ms);
                let _ = tz; // AMF3 dates have no timezone field
            }
            AmfValue::Array { dense, assoc } => {
                self.write_u8(0x09);
                self.write_u29(((dense.len() as u32) << 1) | 1);
                for (k, v) in assoc {
                    self.write_string(k);
                    self.write_value(v);
                }
                self.write_string("");
                for v in dense {
                    self.write_value(v);
                }
            }
            AmfValue::Object {
                class,
                sealed,
                dynamic,
            } => {
                self.write_u8(0x0A);
                let sealed_count = sealed.len() as u32;
                let has_dynamic = !dynamic.is_empty();
                // bits: sealed_count[4+] | dynamic[3] | not_ext[2]=0 | inline_traits[1] | inline[0]
                let flags = (sealed_count << 4)
                    | (if has_dynamic { 0x08 } else { 0 })
                    | 0x03; // inline traits + inline object (bits 0-1)
                self.write_u29(flags);
                self.write_string(class);
                for (name, _) in sealed {
                    self.write_string(name);
                }
                for (_, val) in sealed {
                    self.write_value(val);
                }
                if has_dynamic {
                    for (k, v) in dynamic {
                        self.write_string(k);
                        self.write_value(v);
                    }
                    self.write_string("");
                }
            }
        }
    }
}

// --- AMF0 Reader ---

pub struct Amf0Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Amf0Reader<'a> {
    pub fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err(format!("unexpected EOF at {}", self.pos));
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u16_be(&mut self) -> Result<u16, String> {
        if self.pos + 2 > self.data.len() {
            return Err(format!("unexpected EOF reading u16 at {}", self.pos));
        }
        let val = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    fn read_i16_be(&mut self) -> Result<i16, String> {
        if self.pos + 2 > self.data.len() {
            return Err(format!("unexpected EOF reading i16 at {}", self.pos));
        }
        let val = i16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(val)
    }

    fn read_u32_be(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.data.len() {
            return Err(format!("unexpected EOF reading u32 at {}", self.pos));
        }
        let val = u32::from_be_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(val)
    }

    fn read_f64_be(&mut self) -> Result<f64, String> {
        if self.pos + 8 > self.data.len() {
            return Err(format!("unexpected EOF reading f64 at {}", self.pos));
        }
        let val = f64::from_be_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(val)
    }

    fn read_string_short(&mut self) -> Result<String, String> {
        let len = self.read_u16_be()? as usize;
        if self.pos + len > self.data.len() {
            return Err(format!("unexpected EOF reading string at {}", self.pos));
        }
        let s = String::from_utf8(self.data[self.pos..self.pos + len].to_vec())
            .map_err(|e| format!("invalid UTF-8 at {}: {e}", self.pos))?;
        self.pos += len;
        Ok(s)
    }

    fn read_string_long(&mut self) -> Result<String, String> {
        let len = self.read_u32_be()? as usize;
        if self.pos + len > self.data.len() {
            return Err(format!("unexpected EOF reading long string at {}", self.pos));
        }
        let s = String::from_utf8(self.data[self.pos..self.pos + len].to_vec())
            .map_err(|e| format!("invalid UTF-8 at {}: {e}", self.pos))?;
        self.pos += len;
        Ok(s)
    }

    /// Read key-value pairs until 00 00 09 end marker.
    fn read_object_pairs(&mut self) -> Result<Vec<(String, AmfValue)>, String> {
        let mut pairs = Vec::new();
        loop {
            let key = self.read_string_short()?;
            if key.is_empty() {
                let end = self.read_u8()?;
                if end != 0x09 {
                    return Err(format!(
                        "expected object end marker 0x09, got 0x{:02x}",
                        end
                    ));
                }
                break;
            }
            let val = self.read_value()?;
            pairs.push((key, val));
        }
        Ok(pairs)
    }

    pub fn read_value(&mut self) -> Result<AmfValue, String> {
        let marker = self.read_u8()?;
        match marker {
            0x00 => {
                // Number (f64 BE)
                let val = self.read_f64_be()?;
                Ok(AmfValue::Double(val))
            }
            0x01 => {
                // Boolean
                let val = self.read_u8()?;
                Ok(AmfValue::Bool(val != 0))
            }
            0x02 => {
                // String (u16 len)
                let s = self.read_string_short()?;
                Ok(AmfValue::String(s))
            }
            0x03 => {
                // Object
                let pairs = self.read_object_pairs()?;
                Ok(AmfValue::Object {
                    class: String::new(),
                    sealed: Vec::new(),
                    dynamic: pairs,
                })
            }
            0x05 => Ok(AmfValue::Null),
            0x06 => Ok(AmfValue::Undefined),
            0x08 => {
                // ECMA Array (u32 count hint + key-value pairs until end marker)
                let _count = self.read_u32_be()?;
                let pairs = self.read_object_pairs()?;
                Ok(AmfValue::Array {
                    dense: Vec::new(),
                    assoc: pairs,
                })
            }
            0x0A => {
                // Strict Array (u32 count + sequential values)
                let count = self.read_u32_be()? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(self.read_value()?);
                }
                Ok(AmfValue::Array {
                    dense: items,
                    assoc: Vec::new(),
                })
            }
            0x0B => {
                // Date (f64 BE millis + i16 BE timezone)
                let ms = self.read_f64_be()?;
                let tz = self.read_i16_be()?;
                Ok(AmfValue::Date(ms, tz))
            }
            0x0C => {
                // Long String (u32 len)
                let s = self.read_string_long()?;
                Ok(AmfValue::String(s))
            }
            other => Err(format!(
                "unsupported AMF0 marker 0x{:02x} at position {}",
                other,
                self.pos - 1
            )),
        }
    }
}

// --- AMF0 Writer ---

pub struct Amf0Writer {
    pub buf: Vec<u8>,
}

impl Amf0Writer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    fn write_u8(&mut self, val: u8) {
        self.buf.push(val);
    }

    fn write_u16_be(&mut self, val: u16) {
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    fn write_i16_be(&mut self, val: i16) {
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    fn write_u32_be(&mut self, val: u32) {
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    fn write_f64_be(&mut self, val: f64) {
        self.buf.extend_from_slice(&val.to_be_bytes());
    }

    fn write_string_short(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.write_u16_be(bytes.len() as u16);
        self.buf.extend_from_slice(bytes);
    }

    /// Write key-value pairs + end marker (00 00 09)
    fn write_object_pairs(&mut self, pairs: &[(String, AmfValue)]) {
        for (key, val) in pairs {
            self.write_string_short(key);
            self.write_value(val);
        }
        self.write_u16_be(0); // empty key
        self.write_u8(0x09); // end marker
    }

    pub fn write_value(&mut self, val: &AmfValue) {
        match val {
            AmfValue::Undefined => self.write_u8(0x06),
            AmfValue::Null => self.write_u8(0x05),
            AmfValue::Bool(b) => {
                self.write_u8(0x01);
                self.write_u8(if *b { 1 } else { 0 });
            }
            AmfValue::Integer(i) => {
                // AMF0 has no integer type; encode as number (f64)
                self.write_u8(0x00);
                self.write_f64_be(*i as f64);
            }
            AmfValue::Double(d) => {
                self.write_u8(0x00);
                self.write_f64_be(*d);
            }
            AmfValue::String(s) => {
                if s.len() > 0xFFFF {
                    self.write_u8(0x0C); // long string
                    self.write_u32_be(s.len() as u32);
                    self.buf.extend_from_slice(s.as_bytes());
                } else {
                    self.write_u8(0x02);
                    self.write_string_short(s);
                }
            }
            AmfValue::Date(ms, tz) => {
                self.write_u8(0x0B);
                self.write_f64_be(*ms);
                self.write_i16_be(*tz);
            }
            AmfValue::Array { dense, assoc } => {
                if !assoc.is_empty() {
                    // ECMA Array
                    self.write_u8(0x08);
                    self.write_u32_be(assoc.len() as u32);
                    self.write_object_pairs(assoc);
                } else {
                    // Strict Array
                    self.write_u8(0x0A);
                    self.write_u32_be(dense.len() as u32);
                    for v in dense {
                        self.write_value(v);
                    }
                }
            }
            AmfValue::Object {
                sealed, dynamic, ..
            } => {
                self.write_u8(0x03);
                let all: Vec<_> = sealed.iter().chain(dynamic.iter()).collect();
                for (key, val) in all {
                    self.write_string_short(key);
                    self.write_value(val);
                }
                self.write_u16_be(0);
                self.write_u8(0x09);
            }
        }
    }
}

// --- SOL file parse/write ---

#[derive(Debug, Clone)]
pub struct SolFile {
    pub name: String,
    pub amf_version: u32,
    pub pairs: Vec<(String, AmfValue)>,
}

/// Parse a complete .sol file from bytes.
pub fn parse_sol(data: &[u8]) -> Result<SolFile, String> {
    if data.len() < 18 {
        return Err("file too small for SOL header".into());
    }

    if data[0] != 0x00 || data[1] != 0xBF {
        return Err(format!("bad magic: {:02x} {:02x}", data[0], data[1]));
    }

    let mut reader = AmfReader::new(data);
    reader.pos = 2;

    let _length = reader.read_u32_be()?;
    let sig = reader.read_bytes(4)?;
    if sig != b"TCSO" {
        return Err(format!("bad signature: {:?}", sig));
    }

    let _marker = reader.read_u16_be()?;
    let _padding = reader.read_u32_be()?;
    let name_len = reader.read_u16_be()? as usize;
    let name_bytes = reader.read_bytes(name_len)?;
    let name = String::from_utf8(name_bytes.to_vec())
        .map_err(|e| format!("invalid UTF-8 in SOL name: {e}"))?;
    let amf_version = reader.read_u32_be()?;
    let body_start = reader.pos;

    let pairs = match amf_version {
        0 => {
            // AMF0: u16 key_len + key + AMF0 value + 0x00 terminator
            let mut amf0 = Amf0Reader::new(data, body_start);
            let mut pairs = Vec::new();
            while amf0.remaining() > 0 {
                let key = amf0.read_string_short()?;
                let value = amf0.read_value()?;
                if amf0.remaining() > 0 {
                    let term = amf0.read_u8()?;
                    if term != 0x00 {
                        return Err(format!(
                            "expected AMF0 pair terminator 0x00, got 0x{:02x} after key '{}'",
                            term, key
                        ));
                    }
                }
                pairs.push((key, value));
            }
            pairs
        }
        3 => {
            // AMF3: AMF3_string key + AMF3 value + 0x00 terminator
            let mut pairs = Vec::new();
            while reader.remaining() > 0 {
                let key = reader.read_string()?;
                let value = reader.read_value()?;
                if reader.remaining() > 0 {
                    let term = reader.read_u8()?;
                    if term != 0x00 {
                        return Err(format!(
                            "expected AMF3 pair terminator 0x00, got 0x{:02x} after key '{}'",
                            term, key
                        ));
                    }
                }
                pairs.push((key, value));
            }
            pairs
        }
        v => {
            return Err(format!("unsupported AMF version {v} (supported: 0, 3)"));
        }
    };

    Ok(SolFile {
        name,
        amf_version,
        pairs,
    })
}

/// Serialize a SolFile back to .sol binary format.
pub fn write_sol(sol: &SolFile) -> Vec<u8> {
    // Write body first to compute length
    let body_bytes = match sol.amf_version {
        0 => {
            let mut body = Amf0Writer::new();
            for (key, value) in &sol.pairs {
                body.write_string_short(key);
                body.write_value(value);
                body.buf.push(0x00);
            }
            body.into_bytes()
        }
        _ => {
            let mut body = AmfWriter::new();
            for (key, value) in &sol.pairs {
                body.write_string(key);
                body.write_value(value);
                body.buf.push(0x00);
            }
            body.into_bytes()
        }
    };

    // Header
    let mut header = Vec::new();
    header.push(0x00);
    header.push(0xBF);

    let name_bytes = sol.name.as_bytes();
    let total_len = 4 + 2 + 4 + 2 + name_bytes.len() + 4 + body_bytes.len();
    header.extend_from_slice(&(total_len as u32).to_be_bytes());
    header.extend_from_slice(b"TCSO");
    header.extend_from_slice(&0x0004u16.to_be_bytes());
    header.extend_from_slice(&0u32.to_be_bytes());
    header.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
    header.extend_from_slice(name_bytes);
    header.extend_from_slice(&sol.amf_version.to_be_bytes());

    header.extend_from_slice(&body_bytes);
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_real_sol_file() {
        let data = std::fs::read("../test-data/CoC_1.sol")
            .or_else(|_| std::fs::read("D:/Personalisation/Avatar/saves/Corruption of Champions/Saves/CoC_1.sol"))
            .expect("test .sol file not found");

        let sol = parse_sol(&data).expect("failed to parse SOL");
        assert_eq!(sol.name, "CoC_1");
        assert!(!sol.pairs.is_empty(), "should have parsed some pairs");

        // Check known values from the file
        let find = |key: &str| sol.pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v);

        // gender = int 1
        match find("gender") {
            Some(AmfValue::Integer(1)) => {}
            other => panic!("expected gender=Integer(1), got {:?}", other),
        }

        // hairColor = string "black"
        match find("hairColor") {
            Some(AmfValue::String(s)) if s == "black" => {}
            other => panic!("expected hairColor=String(\"black\"), got {:?}", other),
        }

        // short = string "Patrick"
        match find("short") {
            Some(AmfValue::String(s)) if s == "Patrick" => {}
            other => panic!("expected short=String(\"Patrick\"), got {:?}", other),
        }

        // tallness = int 71
        match find("tallness") {
            Some(AmfValue::Integer(71)) => {}
            other => panic!("expected tallness=Integer(71), got {:?}", other),
        }

        // foundMountain = false (Bool)
        match find("foundMountain") {
            Some(AmfValue::Bool(false)) => {}
            other => panic!("expected foundMountain=Bool(false), got {:?}", other),
        }

        eprintln!("Parsed {} key-value pairs from CoC_1.sol", sol.pairs.len());
    }

    #[test]
    fn roundtrip_real_sol_file() {
        let data = std::fs::read("../test-data/CoC_1.sol")
            .or_else(|_| std::fs::read("D:/Personalisation/Avatar/saves/Corruption of Champions/Saves/CoC_1.sol"))
            .expect("test .sol file not found");

        let sol = parse_sol(&data).expect("failed to parse SOL");
        let written = write_sol(&sol);

        // Re-parse the written data
        let sol2 = parse_sol(&written).expect("failed to re-parse written SOL");
        assert_eq!(sol2.name, sol.name);
        assert_eq!(sol2.pairs.len(), sol.pairs.len());

        // Compare JSON representations for semantic equality
        for (i, ((k1, v1), (k2, v2))) in sol.pairs.iter().zip(sol2.pairs.iter()).enumerate() {
            assert_eq!(k1, k2, "key mismatch at pair {i}");
            assert_eq!(v1.to_json(), v2.to_json(), "value mismatch at key '{k1}'");
        }
    }

    #[test]
    fn parse_amf0_sol_file() {
        let data = std::fs::read(
            "C:/Users/Patrick/AppData/Roaming/Macromedia/Flash Player/#SharedObjects/ZWC7JDLT/localhost/MARDEKv3__options.sol",
        );
        if let Ok(data) = data {
            let sol = parse_sol(&data).expect("failed to parse AMF0 SOL");
            assert_eq!(sol.amf_version, 0);
            assert!(!sol.pairs.is_empty());

            // Known: OBJ key with nested object containing "music" = 1.0
            let find = |key: &str| sol.pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v);
            match find("OBJ") {
                Some(AmfValue::Object { dynamic, .. }) => {
                    let music = dynamic.iter().find(|(k, _)| k == "music");
                    match music {
                        Some((_, AmfValue::Double(v))) if (*v - 1.0).abs() < 0.001 => {}
                        other => panic!("expected music=1.0, got {:?}", other),
                    }
                }
                other => panic!("expected OBJ=Object, got {:?}", other),
            }

            // Roundtrip
            let written = write_sol(&sol);
            let sol2 = parse_sol(&written).expect("failed to re-parse AMF0 SOL");
            assert_eq!(sol2.pairs.len(), sol.pairs.len());
            for ((k1, v1), (k2, v2)) in sol.pairs.iter().zip(sol2.pairs.iter()) {
                assert_eq!(k1, k2);
                assert_eq!(v1.to_json(), v2.to_json(), "mismatch at key '{k1}'");
            }

            eprintln!("AMF0 test passed: {} pairs from MARDEKv3__options.sol", sol.pairs.len());
        } else {
            eprintln!("Skipping AMF0 test (MARDEK file not found)");
        }
    }

    #[test]
    fn parse_amf0_large_sol() {
        let data = std::fs::read(
            "C:/Users/Patrick/AppData/Roaming/Macromedia/Flash Player/#SharedObjects/ZWC7JDLT/localhost/MARDEKv3__sg_0.sol",
        );
        if let Ok(data) = data {
            let sol = parse_sol(&data).expect("failed to parse AMF0 save");
            assert_eq!(sol.amf_version, 0);
            assert!(!sol.pairs.is_empty());
            eprintln!("AMF0 large test passed: {} pairs, {} bytes", sol.pairs.len(), data.len());

            // Roundtrip
            let written = write_sol(&sol);
            let sol2 = parse_sol(&written).expect("roundtrip failed");
            assert_eq!(sol2.pairs.len(), sol.pairs.len());
        } else {
            eprintln!("Skipping AMF0 large test (MARDEK sg_0 not found)");
        }
    }

    #[test]
    fn u29_roundtrip() {
        let test_values: &[u32] = &[0, 1, 0x7F, 0x80, 0x3FFF, 0x4000, 0x1FFFFF, 0x200000, 0x1FFFFFFF];
        for &val in test_values {
            let mut writer = AmfWriter::new();
            writer.write_u29(val);
            let bytes = writer.into_bytes();

            let mut reader = AmfReader::new(&bytes);
            let result = reader.read_u29().unwrap();
            assert_eq!(result, val, "U29 roundtrip failed for {val}");
        }
    }
}
