use serde::{Deserialize, Serialize};
use named_type_derive::*;
use named_type::NamedType;

#[derive(Serialize, Deserialize, Debug, NamedType)]
pub struct TestStruct {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub string: String, 
    pub _char: char,
    pub _u8 : u8,
    // May Need a BigInt Value in PostGres
    pub _u16 : u16,
    pub _u32 : u32,
    // pub _u64 : u64,
    // pub _usize: usize,
    pub _i8: i8,
    pub _i16: i16,
    pub _i32: i32,
    pub _i64: i64,
    // pub _isize: isize,
    pub _f32 : f32,
    pub _f64 : f64
}

// pub struct TestManyToOne {
//     test_struct_id: u64,
//     value: String
// }

impl TestStruct{
    #[allow(dead_code)]
    pub fn new() -> Self {
        TestStruct {
            id: None,
            string: "String".to_string(), 
            _char: 'c',
            _u8 : u8::MAX,
            _u16 : u16::MAX,
            _u32 : u32::MAX,
            _i8: i8::MAX,
            _i16: i16::MAX,
            _i32: i32::MAX,
            _i64: i64::MAX,
            _f32 : f32::MAX,
            _f64 : f64::MAX
        }
    }
    #[allow(dead_code)]
    pub fn new_min() -> Self {
        TestStruct {
            id: None,
            string: "String".to_string(), 
            _char: 'c',
            _u8 : u8::MIN,
            _u16 : u16::MIN,
            _u32 : u32::MIN,
            _i8: i8::MIN,
            _i16: i16::MIN,
            _i32: i32::MIN,
            _i64: i64::MIN,
            _f32 : f32::MIN,
            _f64 : f64::MIN
        }
    }
}