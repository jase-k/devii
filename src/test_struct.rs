use serde::{Deserialize, Serialize};
use serde::de::{Deserializer};
use named_type_derive::*;
use named_type::NamedType;
use serde_json::Value;

use crate::devii::{DeviiTrait};

#[derive(Serialize, Deserialize, Debug, NamedType, Default)]
pub struct TestStruct {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    #[serde(skip_serializing)]
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
    pub _f64 : f64,
}

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

impl DeviiTrait for TestStruct {
    fn fetch_fields() -> String {
        format!("{{ id, string, _char, _u8, _u16, _u32, _i8, _i16, _i32, _i64, _f32, _f64 }}")
    }
    fn insert_query(&self, param: String) -> String{
        format!("create_test_struct (input: ${} ){{ id }}", param)
    }
    fn input_type(&self) -> String {
        "test_structInput".to_string()
    }
    fn graphql_inputs(&self) -> serde_json::Value {
        serde_json::to_value(&self).unwrap()
    }
    fn delete_input(&self) -> String {
        format!("id: {}", self.id.unwrap())
    }
}


// Credit : https://noyez.gitlab.io/post/2018-08-28-serilize-this-or-that-into-u64/
#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrU64 { U64(u64), Str(String) }
pub fn deserialize_u64_or_string<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where D: Deserializer<'de>
{
    match StringOrU64::deserialize(deserializer)? {
        StringOrU64::U64(v) => { Ok(Some(v)) }
        StringOrU64::Str(v) => {
            let res = v.parse::<u64>();
            if let Ok(r) = res {
                Ok(Some(r))
            } else {
                Err(serde::de::Error::custom("Can't parse id!"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, NamedType, Default)]
pub struct TestOneToMany {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub id: Option<u64>,
    pub value: String,
    pub test_many_to_one_collection : Option<Vec<TestManyToOne>>
}


impl DeviiTrait for TestOneToMany {
    fn fetch_fields() -> String {
        format!("{{ id, value, test_many_to_one_collection {} }}", TestManyToOne::fetch_fields())
    }
    fn insert_query(&self, param: String) -> String{
        format!("create_test_one_to_many (input: ${} ){{ id }}", param)
    }
    fn input_type(&self) -> String {
        "test_one_to_manyInput".to_string()
    }
    fn graphql_inputs(&self) -> serde_json::Value {
        let value = serde_json::to_value(&self).unwrap();

        match value {
            Value::Object(mut map) => {
                map.remove_entry("test_many_to_one_collection");
                return Value::Object(map);
            },
            _ => panic!("object wasn't a map!"),
        }
    }
    fn delete_input(&self) -> String {
        format!("id: {}", self.id.unwrap())
    }
}

#[derive(Serialize, Deserialize, Debug, NamedType, Default)]
pub struct TestManyToOne {
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    // #[serde(skip_serializing)]
    pub id: Option<u64>,
    #[serde(deserialize_with = "deserialize_u64_or_string")]
    pub test_one_to_many_id: Option<u64>,
    pub test_one_to_many: Option<TestOneToMany>,
    pub value: String
}

impl DeviiTrait for TestManyToOne {
    fn fetch_fields() -> String {
        format!("{{ id, value, test_one_to_many_id, test_one_to_many {{ id, value }} }}")
    }
    fn insert_query(&self, param: String) -> String{
        format!("create_test_many_to_one (input: ${} ){{ id }}", param)
    }
    fn input_type(&self) -> String {
        "test_many_to_oneInput".to_string()
    }
    fn graphql_inputs(&self) -> serde_json::Value {
        let value = serde_json::to_value(&self).unwrap();

        match value {
            Value::Object(mut map) => {
                map.remove_entry("test_one_to_many");
                return Value::Object(map);
            },
            _ => panic!("object wasn't a map!"),
        }
    }
    fn delete_input(&self) -> String {
        format!("id: {}", self.id.unwrap())
    }
}

impl TestOneToMany {
    #[allow(dead_code)]
    pub fn new() -> Self {
        TestOneToMany{
            id: None, 
            value: "OneToMany".to_string(),
            test_many_to_one_collection: Some(vec![
                TestManyToOne {
                    id: None,
                    test_one_to_many_id: None, 
                    value: "Hello World from Layer 2~".to_string(),
                    test_one_to_many: None
                },
                TestManyToOne {
                    id: None,
                    test_one_to_many_id: None, 
                    value: "I wouldn't have said that...".to_string(),
                    test_one_to_many: None
                },
            ])
        }
    }
}