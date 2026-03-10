/// Custom pickle protocol 2 parser for Ren'Py save files.
///
/// serde-pickle can't handle Ren'Py saves because it replaces unknown globals
/// with empty dicts, but RevertableList/RevertableSet are list/set subclasses
/// that need APPENDS to work. This parser handles those types natively.

use serde_json::{json, Map, Value};
use std::collections::HashMap;

// Pickle opcodes we need for protocol 2
const PROTO: u8 = 0x80;
const STOP: u8 = b'.';
const MARK: u8 = b'(';
const POP: u8 = b'0';
const POP_MARK: u8 = b'1';
const DUP: u8 = b'2';
const NONE: u8 = b'N';
const NEWTRUE: u8 = 0x88;
const NEWFALSE: u8 = 0x89;
const INT: u8 = b'I';
const BININT: u8 = b'J';
const BININT1: u8 = b'K';
const BININT2: u8 = b'M';
const LONG1: u8 = 0x8a;
const FLOAT: u8 = b'F';
const BINFLOAT: u8 = b'G';
const STRING: u8 = b'S';
const BINSTRING: u8 = b'T';
const SHORT_BINSTRING: u8 = b'U';
const BINUNICODE: u8 = b'X';
// SHORT_BINUNICODE handled via SHORT_BINUNICODE_ alias below
const BINBYTES: u8 = b'B';
// SHORT_BINBYTES handled via SHORT_BINBYTES_ alias below
const EMPTY_LIST: u8 = b']';
const EMPTY_TUPLE: u8 = b')';
const EMPTY_DICT: u8 = b'}';
const LIST: u8 = b'l';
const TUPLE: u8 = b't';
const DICT: u8 = b'd';
const TUPLE1: u8 = 0x85;
const TUPLE2: u8 = 0x86;
const TUPLE3: u8 = 0x87;
const APPEND: u8 = b'a';
const APPENDS: u8 = b'e';
const SETITEM: u8 = b's';
const SETITEMS: u8 = b'u';
const BINGET: u8 = b'h';
const LONG_BINGET: u8 = b'j';
const BINPUT: u8 = b'q';
const LONG_BINPUT: u8 = b'r';
const GLOBAL: u8 = b'c';
const NEWOBJ: u8 = 0x81;
const REDUCE: u8 = b'R';
const BUILD: u8 = b'b';
const INST: u8 = b'i';
const OBJ: u8 = b'o';
// SETITEM_ is same as SETITEM
const BINPERSID: u8 = b'Q';
const EMPTY_SET: u8 = 0x8f;
const ADDITEMS: u8 = 0x90;
const FROZENSET: u8 = 0x91;
const MEMOIZE: u8 = 0x94;
const FRAME: u8 = 0x95;
const SHORT_BINUNICODE_: u8 = 0x8c;
const STACK_GLOBAL: u8 = 0x93;
const SHORT_BINBYTES_: u8 = b'C';
const BINUNICODE8: u8 = 0x8d;
const LONG4: u8 = 0x8b;

/// Internal value type that preserves list vs dict vs object distinction
/// during parsing, then converts to serde_json::Value at the end.
#[derive(Debug, Clone)]
enum PVal {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<PVal>),
    Tuple(Vec<PVal>),
    Dict(Vec<(PVal, PVal)>),
    /// A class instance: (class_name, attributes_dict or state)
    Object(String, Box<PVal>),
    /// A callable reference (GLOBAL): module.name
    Global(String),
    /// Set of items
    Set(Vec<PVal>),
}

impl PVal {
    fn to_json(self) -> Value {
        match self {
            PVal::None => Value::Null,
            PVal::Bool(b) => Value::Bool(b),
            PVal::Int(n) => json!(n),
            PVal::Float(f) => json!(f),
            PVal::Str(s) => Value::String(s),
            PVal::Bytes(b) => {
                // Try UTF-8 first, fall back to base64
                match String::from_utf8(b.clone()) {
                    Ok(s) => Value::String(s),
                    Err(_) => Value::String(format!(
                        "b64:{}",
                        base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &b
                        )
                    )),
                }
            }
            PVal::List(items) | PVal::Tuple(items) | PVal::Set(items) => {
                Value::Array(items.into_iter().map(|v| v.to_json()).collect())
            }
            PVal::Dict(pairs) => {
                let mut map = Map::new();
                for (k, v) in pairs {
                    let key = match k {
                        PVal::Str(s) => s,
                        PVal::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                        PVal::Int(n) => n.to_string(),
                        PVal::Float(f) => f.to_string(),
                        PVal::Bool(b) => b.to_string(),
                        PVal::None => "None".to_string(),
                        other => format!("{:?}", other),
                    };
                    map.insert(key, v.to_json());
                }
                Value::Object(map)
            }
            PVal::Object(class_name, state) => {
                // Convert state to JSON, merge class name in
                match *state {
                    PVal::Dict(pairs) => {
                        let mut map = Map::new();
                        map.insert("__class__".into(), Value::String(class_name));
                        for (k, v) in pairs {
                            let key = match k {
                                PVal::Str(s) => s,
                                PVal::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                                PVal::Int(n) => n.to_string(),
                                other => format!("{:?}", other),
                            };
                            map.insert(key, v.to_json());
                        }
                        Value::Object(map)
                    }
                    PVal::None => {
                        let mut map = Map::new();
                        map.insert("__class__".into(), Value::String(class_name));
                        Value::Object(map)
                    }
                    other => {
                        let mut map = Map::new();
                        map.insert("__class__".into(), Value::String(class_name));
                        map.insert("__state__".into(), other.to_json());
                        Value::Object(map)
                    }
                }
            }
            PVal::Global(name) => Value::String(format!("<global:{name}>")),
        }
    }

}

struct PickleVM<'a> {
    data: &'a [u8],
    pos: usize,
    stack: Vec<PVal>,
    memo: HashMap<u32, PVal>,
    /// Mark stack — positions in `stack` where MARK was placed
    marks: Vec<usize>,
}

impl<'a> PickleVM<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            stack: Vec::new(),
            memo: HashMap::new(),
            marks: Vec::new(),
        }
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.data.len() {
            return Err("unexpected EOF".into());
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_u16_le(&mut self) -> Result<u16, String> {
        if self.pos + 2 > self.data.len() {
            return Err("unexpected EOF reading u16".into());
        }
        let v = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn read_i32_le(&mut self) -> Result<i32, String> {
        if self.pos + 4 > self.data.len() {
            return Err("unexpected EOF reading i32".into());
        }
        let v = i32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_u32_le(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.data.len() {
            return Err("unexpected EOF reading u32".into());
        }
        let v = u32::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_u64_le(&mut self) -> Result<u64, String> {
        if self.pos + 8 > self.data.len() {
            return Err("unexpected EOF reading u64".into());
        }
        let v = u64::from_le_bytes([
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
            self.data[self.pos + 4],
            self.data[self.pos + 5],
            self.data[self.pos + 6],
            self.data[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(v)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.pos + n > self.data.len() {
            return Err(format!("unexpected EOF reading {n} bytes at 0x{:X}", self.pos));
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_line(&mut self) -> Result<&'a [u8], String> {
        let start = self.pos;
        while self.pos < self.data.len() {
            if self.data[self.pos] == b'\n' {
                let line = &self.data[start..self.pos];
                self.pos += 1; // skip newline
                return Ok(line);
            }
            self.pos += 1;
        }
        Err("unexpected EOF reading line".into())
    }

    fn pop(&mut self) -> Result<PVal, String> {
        self.stack.pop().ok_or_else(|| "stack underflow".into())
    }

    fn top_mut(&mut self) -> Result<&mut PVal, String> {
        self.stack.last_mut().ok_or_else(|| "stack empty".into())
    }

    /// Pop items from stack back to the last MARK
    fn pop_mark(&mut self) -> Result<Vec<PVal>, String> {
        let mark_pos = self.marks.pop().ok_or("no mark on stack")?;
        let items = self.stack.split_off(mark_pos);
        Ok(items)
    }

    /// Determine what base type a global should map to
    fn global_base_type(module: &str, name: &str) -> PVal {
        match (module, name) {
            // Ren'Py revertable types → map to their Python base types
            ("renpy.revertable", "RevertableList") => PVal::List(Vec::new()),
            ("renpy.revertable", "RevertableDict") => PVal::Dict(Vec::new()),
            ("renpy.revertable", "RevertableSet") => PVal::Set(Vec::new()),
            // Python builtins
            ("builtins", "list") | ("__builtin__", "list") => PVal::List(Vec::new()),
            ("builtins", "dict") | ("__builtin__", "dict") => PVal::Dict(Vec::new()),
            ("builtins", "set") | ("__builtin__", "set") => PVal::Set(Vec::new()),
            ("builtins", "frozenset") | ("__builtin__", "frozenset") => PVal::Set(Vec::new()),
            ("builtins", "bytearray") | ("__builtin__", "bytearray") => PVal::Bytes(Vec::new()),
            // copyreg._reconstructor — will be handled by REDUCE
            ("copyreg", "_reconstructor") | ("copy_reg", "_reconstructor") => {
                PVal::Global(format!("{module}.{name}"))
            }
            // Everything else — store as a callable global reference
            _ => PVal::Global(format!("{module}.{name}")),
        }
    }

    /// Execute the pickle VM and return the result
    fn execute(mut self) -> Result<PVal, String> {
        loop {
            if self.pos >= self.data.len() {
                return Err("unexpected end of pickle stream".into());
            }
            let opcode = self.read_u8()?;

            match opcode {
                PROTO => {
                    let _version = self.read_u8()?;
                }

                STOP => {
                    return self.pop();
                }

                FRAME => {
                    // Protocol 4 frame — just skip the 8-byte frame length
                    let _len = self.read_u64_le()?;
                }

                // === Constants ===
                NONE => self.stack.push(PVal::None),
                NEWTRUE => self.stack.push(PVal::Bool(true)),
                NEWFALSE => self.stack.push(PVal::Bool(false)),

                // === Integers ===
                INT => {
                    let line = self.read_line()?;
                    let s = std::str::from_utf8(line)
                        .map_err(|_| "invalid utf8 in INT")?
                        .trim_end_matches('\r');
                    if s == "00" {
                        self.stack.push(PVal::Bool(false));
                    } else if s == "01" {
                        self.stack.push(PVal::Bool(true));
                    } else {
                        let n: i64 = s.parse().map_err(|_| format!("invalid INT: {s}"))?;
                        self.stack.push(PVal::Int(n));
                    }
                }
                BININT => {
                    let n = self.read_i32_le()?;
                    self.stack.push(PVal::Int(n as i64));
                }
                BININT1 => {
                    let n = self.read_u8()?;
                    self.stack.push(PVal::Int(n as i64));
                }
                BININT2 => {
                    let n = self.read_u16_le()?;
                    self.stack.push(PVal::Int(n as i64));
                }
                LONG1 => {
                    let nbytes = self.read_u8()? as usize;
                    let bytes = self.read_bytes(nbytes)?;
                    // Decode as little-endian signed integer
                    let mut n: i64 = 0;
                    for (i, &b) in bytes.iter().enumerate() {
                        n |= (b as i64) << (i * 8);
                    }
                    // Sign extend if the high bit is set
                    if nbytes > 0 && nbytes < 8 && (bytes[nbytes - 1] & 0x80) != 0 {
                        n |= !0i64 << (nbytes * 8);
                    }
                    self.stack.push(PVal::Int(n));
                }
                LONG4 => {
                    let nbytes = self.read_u32_le()? as usize;
                    let bytes = self.read_bytes(nbytes)?;
                    let mut n: i64 = 0;
                    for (i, &b) in bytes.iter().enumerate().take(8) {
                        n |= (b as i64) << (i * 8);
                    }
                    if nbytes > 0 && nbytes < 8 && (bytes[nbytes - 1] & 0x80) != 0 {
                        n |= !0i64 << (nbytes * 8);
                    }
                    self.stack.push(PVal::Int(n));
                }

                // === Floats ===
                FLOAT => {
                    let line = self.read_line()?;
                    let s = std::str::from_utf8(line)
                        .map_err(|_| "invalid utf8 in FLOAT")?
                        .trim_end_matches('\r');
                    let f: f64 = s.parse().map_err(|_| format!("invalid FLOAT: {s}"))?;
                    self.stack.push(PVal::Float(f));
                }
                BINFLOAT => {
                    let bits = self.read_u64_le()?;
                    // BINFLOAT is big-endian in pickle!
                    let be_bytes = bits.to_le_bytes();
                    let f = f64::from_be_bytes(be_bytes);
                    self.stack.push(PVal::Float(f));
                }

                // === Strings / Unicode / Bytes ===
                STRING => {
                    let line = self.read_line()?;
                    let s = std::str::from_utf8(line).map_err(|_| "invalid utf8 in STRING")?;
                    // Strip quotes
                    let s = s.trim_end_matches('\r');
                    let s = if (s.starts_with('\'') && s.ends_with('\''))
                        || (s.starts_with('"') && s.ends_with('"'))
                    {
                        &s[1..s.len() - 1]
                    } else {
                        s
                    };
                    self.stack.push(PVal::Str(s.to_string()));
                }
                SHORT_BINSTRING => {
                    let len = self.read_u8()? as usize;
                    let bytes = self.read_bytes(len)?;
                    // In protocol 2, SHORT_BINSTRING is bytes (Python 2 str)
                    // Try as UTF-8 for compatibility
                    match std::str::from_utf8(bytes) {
                        Ok(s) => self.stack.push(PVal::Str(s.to_string())),
                        Err(_) => self.stack.push(PVal::Bytes(bytes.to_vec())),
                    }
                }
                BINSTRING => {
                    let len = self.read_i32_le()? as usize;
                    let bytes = self.read_bytes(len)?;
                    match std::str::from_utf8(bytes) {
                        Ok(s) => self.stack.push(PVal::Str(s.to_string())),
                        Err(_) => self.stack.push(PVal::Bytes(bytes.to_vec())),
                    }
                }
                BINUNICODE => {
                    let len = self.read_u32_le()? as usize;
                    let bytes = self.read_bytes(len)?;
                    let s = String::from_utf8_lossy(bytes).to_string();
                    self.stack.push(PVal::Str(s));
                }
                SHORT_BINUNICODE_ => {
                    let len = self.read_u8()? as usize;
                    let bytes = self.read_bytes(len)?;
                    let s = String::from_utf8_lossy(bytes).to_string();
                    self.stack.push(PVal::Str(s));
                }
                BINUNICODE8 => {
                    let len = self.read_u64_le()? as usize;
                    let bytes = self.read_bytes(len)?;
                    let s = String::from_utf8_lossy(bytes).to_string();
                    self.stack.push(PVal::Str(s));
                }
                SHORT_BINBYTES_ => {
                    let len = self.read_u8()? as usize;
                    let bytes = self.read_bytes(len)?;
                    self.stack.push(PVal::Bytes(bytes.to_vec()));
                }
                BINBYTES => {
                    let len = self.read_u32_le()? as usize;
                    let bytes = self.read_bytes(len)?;
                    self.stack.push(PVal::Bytes(bytes.to_vec()));
                }

                // === Collections ===
                EMPTY_LIST => self.stack.push(PVal::List(Vec::new())),
                EMPTY_TUPLE => self.stack.push(PVal::Tuple(Vec::new())),
                EMPTY_DICT => self.stack.push(PVal::Dict(Vec::new())),
                EMPTY_SET => self.stack.push(PVal::Set(Vec::new())),

                LIST => {
                    let items = self.pop_mark()?;
                    self.stack.push(PVal::List(items));
                }
                TUPLE => {
                    let items = self.pop_mark()?;
                    self.stack.push(PVal::Tuple(items));
                }
                DICT => {
                    let items = self.pop_mark()?;
                    let mut pairs = Vec::new();
                    for chunk in items.chunks_exact(2) {
                        pairs.push((chunk[0].clone(), chunk[1].clone()));
                    }
                    self.stack.push(PVal::Dict(pairs));
                }

                TUPLE1 => {
                    let a = self.pop()?;
                    self.stack.push(PVal::Tuple(vec![a]));
                }
                TUPLE2 => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(PVal::Tuple(vec![a, b]));
                }
                TUPLE3 => {
                    let c = self.pop()?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(PVal::Tuple(vec![a, b, c]));
                }

                FROZENSET => {
                    let items = self.pop_mark()?;
                    self.stack.push(PVal::Set(items));
                }

                APPEND => {
                    let item = self.pop()?;
                    let list = self.top_mut()?;
                    match list {
                        PVal::List(items) => items.push(item),
                        PVal::Object(_, ref mut inner) => {
                            if let PVal::List(items) = inner.as_mut() {
                                items.push(item);
                            }
                        }
                        // Tolerate: treat as dict with numeric keys or just skip
                        _ => {}
                    }
                }
                APPENDS => {
                    let items = self.pop_mark()?;
                    let list = self.top_mut()?;
                    match list {
                        PVal::List(existing) => existing.extend(items),
                        PVal::Object(_, ref mut inner) => {
                            if let PVal::List(existing) = inner.as_mut() {
                                existing.extend(items);
                            }
                        }
                        _ => {}
                    }
                }

                SETITEM => {
                    let val = self.pop()?;
                    let key = self.pop()?;
                    let dict = self.top_mut()?;
                    match dict {
                        PVal::Dict(pairs) => pairs.push((key, val)),
                        PVal::Object(_, ref mut inner) => {
                            if let PVal::Dict(pairs) = inner.as_mut() {
                                pairs.push((key, val));
                            }
                        }
                        _ => {}
                    }
                }
                SETITEMS => {
                    let items = self.pop_mark()?;
                    let dict = self.top_mut()?;
                    let pairs_to_add: Vec<(PVal, PVal)> = items
                        .chunks_exact(2)
                        .map(|c| (c[0].clone(), c[1].clone()))
                        .collect();
                    match dict {
                        PVal::Dict(pairs) => pairs.extend(pairs_to_add),
                        PVal::Object(_, ref mut inner) => {
                            if let PVal::Dict(pairs) = inner.as_mut() {
                                pairs.extend(pairs_to_add);
                            }
                        }
                        _ => {}
                    }
                }

                ADDITEMS => {
                    let items = self.pop_mark()?;
                    let set = self.top_mut()?;
                    match set {
                        PVal::Set(existing) => existing.extend(items),
                        PVal::Object(_, ref mut inner) => {
                            if let PVal::Set(existing) = inner.as_mut() {
                                existing.extend(items);
                            }
                        }
                        _ => {}
                    }
                }

                // === Mark ===
                MARK => {
                    self.marks.push(self.stack.len());
                }

                POP => {
                    self.pop()?;
                }
                POP_MARK => {
                    let _items = self.pop_mark()?;
                }
                DUP => {
                    let top = self.stack.last().ok_or("stack empty for DUP")?.clone();
                    self.stack.push(top);
                }

                // === Memo ===
                BINPUT => {
                    let idx = self.read_u8()? as u32;
                    let val = self.stack.last().ok_or("stack empty for BINPUT")?.clone();
                    self.memo.insert(idx, val);
                }
                LONG_BINPUT => {
                    let idx = self.read_u32_le()?;
                    let val = self.stack.last().ok_or("stack empty for LONG_BINPUT")?.clone();
                    self.memo.insert(idx, val);
                }
                MEMOIZE => {
                    let idx = self.memo.len() as u32;
                    let val = self.stack.last().ok_or("stack empty for MEMOIZE")?.clone();
                    self.memo.insert(idx, val);
                }
                BINGET => {
                    let idx = self.read_u8()? as u32;
                    let val = self.memo.get(&idx).ok_or_else(|| format!("memo key {idx} not found"))?.clone();
                    self.stack.push(val);
                }
                LONG_BINGET => {
                    let idx = self.read_u32_le()?;
                    let val = self.memo.get(&idx).ok_or_else(|| format!("memo key {idx} not found"))?.clone();
                    self.stack.push(val);
                }

                // === Class / Object ===
                GLOBAL => {
                    let module_line = self.read_line()?;
                    let name_line = self.read_line()?;
                    let module = std::str::from_utf8(module_line)
                        .map_err(|_| "invalid utf8 in GLOBAL module")?
                        .trim_end_matches('\r');
                    let name = std::str::from_utf8(name_line)
                        .map_err(|_| "invalid utf8 in GLOBAL name")?
                        .trim_end_matches('\r');
                    self.stack.push(Self::global_base_type(module, name));
                }

                STACK_GLOBAL => {
                    let name = self.pop()?;
                    let module = self.pop()?;
                    let (m, n) = match (&module, &name) {
                        (PVal::Str(m), PVal::Str(n)) => (m.as_str(), n.as_str()),
                        _ => ("?", "?"),
                    };
                    self.stack.push(Self::global_base_type(m, n));
                }

                NEWOBJ => {
                    let args = self.pop()?;
                    let cls = self.pop()?;
                    // Create instance based on what the class resolved to
                    match cls {
                        PVal::List(_) => self.stack.push(PVal::List(Vec::new())),
                        PVal::Dict(_) => self.stack.push(PVal::Dict(Vec::new())),
                        PVal::Set(_) => self.stack.push(PVal::Set(Vec::new())),
                        PVal::Global(name) => {
                            // Unknown class — create an Object with the args
                            self.stack.push(PVal::Object(name, Box::new(args)));
                        }
                        _ => self.stack.push(PVal::Dict(Vec::new())),
                    }
                }

                REDUCE => {
                    let args = self.pop()?;
                    let callable = self.pop()?;
                    match callable {
                        PVal::Global(ref name) if name == "copyreg._reconstructor" || name == "copy_reg._reconstructor" => {
                            // _reconstructor(cls, base, state)
                            // The result should be base-type instance
                            if let PVal::Tuple(ref items) = args {
                                if items.len() >= 2 {
                                    let class_name = match &items[0] {
                                        PVal::Global(n) => n.clone(),
                                        _ => "unknown".into(),
                                    };
                                    let base = &items[1];
                                    let instance = match base {
                                        PVal::Global(n) if n.contains("list") => PVal::List(Vec::new()),
                                        PVal::Global(n) if n.contains("dict") => PVal::Dict(Vec::new()),
                                        PVal::Global(n) if n.contains("set") => PVal::Set(Vec::new()),
                                        _ => PVal::Dict(Vec::new()),
                                    };
                                    self.stack.push(PVal::Object(class_name, Box::new(instance)));
                                } else {
                                    self.stack.push(PVal::Dict(Vec::new()));
                                }
                            } else {
                                self.stack.push(PVal::Dict(Vec::new()));
                            }
                        }
                        PVal::List(_) => self.stack.push(PVal::List(Vec::new())),
                        PVal::Dict(_) => self.stack.push(PVal::Dict(Vec::new())),
                        PVal::Set(_) => self.stack.push(PVal::Set(Vec::new())),
                        PVal::Global(name) => {
                            self.stack.push(PVal::Object(name, Box::new(args)));
                        }
                        _ => {
                            self.stack.push(PVal::Dict(Vec::new()));
                        }
                    }
                }

                BUILD => {
                    let state = self.pop()?;
                    let obj = self.top_mut()?;
                    match obj {
                        PVal::Object(_, ref mut inner) => {
                            // Merge state into the object
                            match (&mut **inner, state) {
                                (PVal::Dict(pairs), PVal::Dict(new_pairs)) => {
                                    pairs.extend(new_pairs);
                                }
                                (PVal::Dict(pairs), PVal::Tuple(items)) => {
                                    // __setstate__ with tuple: (dict, slots_dict)
                                    if let Some(PVal::Dict(d)) = items.into_iter().next() {
                                        pairs.extend(d);
                                    }
                                }
                                (_, new_state) => {
                                    **inner = new_state;
                                }
                            }
                        }
                        PVal::Dict(pairs) => {
                            if let PVal::Dict(new_pairs) = state {
                                pairs.extend(new_pairs);
                            }
                        }
                        PVal::List(_) => {
                            // BUILD on a list — state is usually a dict of __dict__ attrs
                            // Wrap it as an object to preserve both
                            // (skip for now, list contents are more important)
                        }
                        _ => {
                            // Replace with state
                            *obj = state;
                        }
                    }
                }

                INST => {
                    let module_line = self.read_line()?;
                    let name_line = self.read_line()?;
                    let module = std::str::from_utf8(module_line).unwrap_or("?").trim_end_matches('\r');
                    let name = std::str::from_utf8(name_line).unwrap_or("?").trim_end_matches('\r');
                    let args = self.pop_mark()?;
                    let class_name = format!("{module}.{name}");
                    self.stack.push(PVal::Object(class_name, Box::new(PVal::Tuple(args))));
                }

                OBJ => {
                    let mut args = self.pop_mark()?;
                    let cls = if !args.is_empty() { args.remove(0) } else { PVal::None };
                    let class_name = match cls {
                        PVal::Global(n) => n,
                        _ => "unknown".into(),
                    };
                    self.stack.push(PVal::Object(class_name, Box::new(PVal::Tuple(args))));
                }

                BINPERSID => {
                    let _pid = self.pop()?;
                    // Persistent references — we can't resolve these without the
                    // Ren'Py runtime, so just push a placeholder
                    self.stack.push(PVal::Str("<persistent_ref>".into()));
                }

                _ => {
                    return Err(format!(
                        "unsupported pickle opcode 0x{:02X} at offset 0x{:X}",
                        opcode,
                        self.pos - 1
                    ));
                }
            }
        }
    }
}

/// Parse pickle bytes into a serde_json::Value.
pub fn parse_pickle(data: &[u8]) -> Result<Value, String> {
    let vm = PickleVM::new(data);
    let result = vm.execute()?;
    Ok(result.to_json())
}
