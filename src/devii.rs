//NOTES: 
// Don't serialize many to one or one to many relationships as they won't show in 'TInput' devii object'
// What happens when I want to serialize to JSON to return to web? 
// serialize if: 

use reqwest;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use named_type::NamedType;
use convert_case::{Case, Casing};
use core::fmt::Debug;
use serde_json::{Map, Value};
use easy_error::bail;


pub trait GraphQLQuery{}

pub trait DeviiTrait{
    fn insert_query(&self, param: String) -> String;
    fn input_type(&self) -> String; 
    fn graphql_inputs(&self) -> Value;
    fn fetch_fields() -> String;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiClient {
    access_token: String,
    refresh_token: String,
    message: String,
    routes: DeviiRoutes
}

#[derive(Serialize, Deserialize, Debug)]
struct DeviiRoutes {
    base: String,
    query: String,
    roles_pbac: String
}


#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiClientOptions {
    login: String,
    tenantid: u32,
    password: String,

    #[serde(skip_serializing)]
    base: String
}

impl DeviiClientOptions {
    pub fn new(login: String, password: String, base: String, tenantid: u32) -> Self {
        DeviiClientOptions {
            login,
            tenantid,
            password,
            base
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiQueryOptions {
    pub query: String,
    pub variables: Option<FetchOptions>
}

#[derive(Serialize, Deserialize, Debug, Builder, Default)]
#[builder(setter(strip_option))]
#[builder(default)]
pub struct FetchOptions  {
    filter: Option<String>,
    offset: Option<u64>,
    ordering: Option<Vec<String>>,
    limit: Option<u64>
}

impl GraphQLQuery for DeviiQueryOptions{}

#[derive(Serialize, Debug, Deserialize)]
pub struct DeviiQueryInsertOptions<T: Serialize> {
    pub query: String,
    // Docs: https://serde.rs/attr-bound.html
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    pub variables: Insert<T>
}

#[derive(Serialize, Debug, Deserialize)]
pub struct DeviiQueryBatchInsertOptions {
    pub query: String,
    // Docs: https://serde.rs/attr-bound.html
    // #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    pub variables: String
}
impl GraphQLQuery for DeviiQueryBatchInsertOptions{}

#[derive(Serialize, Debug, Deserialize)]
pub struct DeviiQueryUpdateOptions<T: Serialize> {
    pub query: String,
    // Docs: https://serde.rs/attr-bound.html
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    pub variables: Update<T>
}

impl <T: DeserializeOwned + Serialize>GraphQLQuery for DeviiQueryUpdateOptions<T>{}



#[derive(Serialize, Debug, Deserialize)]
pub struct Insert<T: Serialize> {
    // Docs: https://serde.rs/attr-bound.html
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    input: T
}

#[derive(Serialize, Debug, Deserialize)]
pub struct Update<T: Serialize> {
    // Docs: https://serde.rs/attr-bound.html
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    input: T,
    id: u64 
}

impl <T: DeserializeOwned + Serialize>GraphQLQuery for DeviiQueryInsertOptions<T>{}



impl DeviiClient {
    pub async fn connect(options: DeviiClientOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        let res = client.post(format!("{}/auth", options.base))
            .json(&options)
            .send()
            .await?
            .json::<DeviiClient>()
            .await?;

        Ok(res)
    }

    // Type T has to be DeserializedOwned as required by .json<> when deserializing the result into a Rust Struct
    pub async fn query<T: DeserializeOwned, K : GraphQLQuery + Serialize>(&self, options: &K) -> Result<T, Box< dyn std::error::Error>>
    {
        let client = reqwest::Client::new();
        //Add Auth header
        let res = client.post(&self.routes.query)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&options)
            .build()?;

        let execute_result = client.execute(res)
        .await?;

        let result_text = execute_result.text().await?;
        // let result_text_clone = result_text.clone();
        
        let result = serde_json::from_str(&result_text);    
            // .json::<T>()
            // .await;
        
        match result {
            Ok(r) => return Ok(r),
            Err(e) => bail!("Failed to Parse struct from Result: {:?}, Error: {:?}", result_text, e)
        }
    }

    // returns UniqueIdentifier as string, string. 
    pub async fn insert<T: DeserializeOwned + Serialize + NamedType>(&self, object: &T) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        // create query. 
        let insert_object; 

        if let Value::Object(mut map) = serde_json::to_value(object)? {
            let mut keys  = map.keys();
            let mut keys_to_remove = vec![];
            while let Some(key) = keys.next() {
                if let Some(value) = map.get(key) {
                    match value {
                        Value::Null => keys_to_remove.push(key.clone()),
                        Value::Object(_) => keys_to_remove.push(key.clone()),
                        Value::Array(_) => keys_to_remove.push(key.clone()),
                        _ => continue,
                    };
                };
            };
            
            while let Some(key) = keys_to_remove.pop() {
                map.remove(&key);
            }

            insert_object = map
        } else {
            bail!("Struct not evaluated as an Object!")
        }


        // println!("Input Object: {:?}", insert_object.keys());

        let insert = Insert {
            input : insert_object
        };

        let snake_type = T::short_type_name().to_case(Case::Snake);

        let query_string = format!("mutation insert ($input: {}Input){{
            create_{} (input: $input){{
              id
            }}
          }}",
          snake_type,
          snake_type
        );

        let query = DeviiQueryInsertOptions{ 
            query: query_string,
            variables: insert
        };

        let mut result = self.query::<DeviiQueryResult<HashMap<String, String>>, DeviiQueryInsertOptions<serde_json::map::Map<String, Value>>>(&query).await?;

        let id_from_insert = result.data.remove(&(format!("create_{}", snake_type))).unwrap();
        
        Ok(id_from_insert)
    }

    pub async fn batch_insert<T: DeserializeOwned + Serialize + NamedType + DeviiTrait + Debug>(&self, objects: Vec<&T>) -> Result<String, Box<dyn std::error::Error>> {
        // create query. 
        // create Devii Trait
            // Trait will include insert_query & input_type

        // build inputs object with HashMap u16 Value as below
        // build query by using foreach:(1_input: input_type) foreach insert_query(1)
        let query_string = get_query_string_from_vec(&objects);

        let mut insert_objects: HashMap<String, Value> = HashMap::new(); 
        let mut counter = 0;
        let mut objects_iter = objects.iter();

        // TODO: make more custom and part of the Devii Trait
        while let Some(object) = objects_iter.next(){
            insert_objects.insert(format!("input_{}", counter), object.graphql_inputs());
            counter = counter + 1;
        }


        // println!("Input Object: {:?}", insert_object.keys());

        let query = DeviiQueryBatchInsertOptions{ 
            query: query_string,
            variables: serde_json::to_string(&insert_objects)?
        };

        let query_result = self.query::<DeviiQueryResult<InsertIdResult>, DeviiQueryBatchInsertOptions>(&query).await;

        if let Err(e) = query_result {
            bail!("Failed Query {:?} Error: {:?}", &query, e);
        }

        // let id_from_insert = result.data.remove(&(format!("create_{}", snake_type))).unwrap();
        Ok("success".to_string())
    }

    pub async fn fetch<T: DeserializeOwned + Serialize + NamedType + Default + DeviiTrait>(&self, id: u64) -> Result<T, Box<dyn std::error::Error>> {
        let snake_type = T::short_type_name().to_case(Case::Snake);

        let query_string = format!("query fetch($filter: String){{
            {} (filter: $filter)
              {}
          }}",
          snake_type,
          T::fetch_fields() 
        );
        println!("{}", query_string);

        let fetch_variables = FetchOptionsBuilder::default().filter(format!("id = {}", id)).build().unwrap();
        
        let query = DeviiQueryOptions{ 
            query: query_string,
            variables: Some(fetch_variables)
        };

        let mut result = self.query::<DeviiQueryResult<Vec<T>>, DeviiQueryOptions>(&query).await?;

        let mut data_result = result.data.remove(&(format!("{}", snake_type))).unwrap();
        
        if data_result.len() > 0 {
            return Ok(data_result.swap_remove(0));
        } else {
            bail!("No Type Found with that id")
        }

    }

    pub async fn update<T: DeserializeOwned + Serialize + NamedType+ Default>(&self, object: T, id: u64) -> Result<T, Box<dyn std::error::Error>>{

        let update = Update {
            input : object,
            id: id
        };

        let snake_type = T::short_type_name().to_case(Case::Snake);

        let query_string = format!("mutation update ($input: {}Input, $id: ID!){{
            update_{} (id: $id, input: $input)
            {}
         }}",
          snake_type,
          snake_type,
          parse_value(&serde_json::to_value(T::default()).unwrap(), Some("id".to_string()))
        );

        let query = DeviiQueryUpdateOptions{ 
            query: query_string,
            variables: update
        };

        let mut result = self.query::<DeviiQueryResult<T>, DeviiQueryUpdateOptions<T>>(&query).await?;

        let type_from_update = result.data.remove(&(format!("update_{}", snake_type))).unwrap();
        
        Ok(type_from_update)
    }
}

pub trait DeviiQueryResultType{}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiQueryResult<T> {
    pub data: HashMap<String, T>
}

#[derive(Serialize, Deserialize, Debug)]
struct InsertIdResult {
    // Result comes back from devii as a string but should be a unique identifier
    id: String
}


// May be usuable in the future -> For automatic FetchFields trait
fn parse_value(value: &Value, additional_fields: Option<String>) -> String {
    let mut additional_field_string = "".to_string(); 

    if let Some(s) = additional_fields {
        additional_field_string = format!("{},", s);
    }
    match value {
        Value::Object(map) => { return ["{", additional_field_string.as_str(), parse_object(&map).as_str(), "}"].join("") },
        Value::Array(vec) => { return [ "{", parse_array(&vec).as_str(), "}"].join("") },
        _ =>  return "".to_string()
    }
}

fn parse_object(map: &Map<String, Value>) -> String {
    let mut iter = map.keys();
    let mut first = true;
    let mut map_vals: Vec<String> = vec![];

    while let Some(i) = iter.next() {
            println!("Map Key: {:?}", i);
            println!("Map Value {:?}", map.get(i));
            if let Some(value) = map.get(i) {
                if !first { map_vals.push(",".to_string())}
                first = false;

                map_vals.push(i.to_string());
                map_vals.push(parse_value(value, None));
            }
        }
    map_vals.join("")
}

// TODO: Currently #[serde(skip_serializing_if = "example")] is not supported as this may result in not all the vec params being found.
// Also doesn't work for empty string... 
fn parse_array(vec: &Vec<Value> ) -> String {
    let vec_obj = vec.iter().next();
    if let Some(o) = vec_obj {
        return parse_value(o, None);
    } else {
        return "".to_string();
    }
}


fn get_query_string_from_vec<T: DeviiTrait>(objects: &Vec<&T>) -> String {
    let mut objects_iter = objects.iter();
    let mut query_string_inputs = vec![];
    let mut query_string_definitions = vec![];
    let mut counter = 0;
    
    while let Some(obj) = objects_iter.next() {
        query_string_inputs.push(format!("${}: {}", format!("input_{}", counter), obj.input_type())); // $1 : testInput
        // insert_1: create_test (input: $1) { id }
        query_string_definitions.push(format!("{}: {}", format!("insert_{}", counter), obj.insert_query(format!("input_{}", counter))));
        counter = counter + 1;
    }
    let query_string = format!("mutation insert ({}){{
        {}
      }}",
      query_string_inputs.join(","),
      query_string_definitions.join(",")
    );

    query_string
}

// cargo test foo -- --test-threads 3
#[cfg(test)]
mod tests {
    use dotenv;
    use crate::devii::DeviiClient;
    use crate::devii::DeviiClientOptions;
    use crate::test_struct::{TestStruct, TestOneToMany};
    use crate::devii::parse_value;
    use crate::devii::DeviiTrait;

    #[test]
    fn parse_value_test() {
        let value = serde_json::to_value(TestStruct::default()).unwrap();
        let result = parse_value(&value, None);
        assert_eq!("{_char,_f32,_f64,_i16,_i32,_i64,_i8,_u16,_u32,_u8,string}".to_string()
        , result)
    }
    #[test]
    fn fetch_fields_test() {
        let value = TestOneToMany::fetch_fields();

        assert_eq!("{ id, value, test_many_to_one_collection { id, value, test_one_to_many_id, test_one_to_many { id, value } } }".to_string()
        , value)
    }

    #[test]
    fn client_connect() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };

        let client = tokio_test::block_on(DeviiClient::connect(options));

        if let Ok(_) = client {
            assert!(true);
        }
        if let Err(e) = client {
            println!("Connection Failed to Devii \n\n {:?}", e);
            assert!(false);
        }
    }
    
    #[test]
    fn client_connect_returns_query_url() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };

        let client_result = tokio_test::block_on(DeviiClient::connect(options));

        if let Ok(res) = client_result {
            assert_eq!(res.routes.query, format!("{}/tenant13/query",dotenv::var("DEVII_BASE_URL").unwrap()));
        } else {
            assert!(false);
        }
    }
    #[test]
    fn insert_struct_test_one_to_many_struct() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };

        let one_to_many_struct = TestOneToMany::new();
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();
        
        let result = tokio_test::block_on(client.insert(&one_to_many_struct));
        let layer1_id = result.unwrap().remove("id").unwrap().parse::<u64>().unwrap();
        
        let mut test_many_to_one_collection = one_to_many_struct.test_many_to_one_collection.unwrap();
        let mut iter = test_many_to_one_collection.iter_mut();
        // let layer2_results = vec![];
        while let Some(obj) = iter.next() {
            obj.test_one_to_many_id = Some(layer1_id);
            let result2 = tokio_test::block_on(client.insert(obj));

            if let Ok(_) = result2 {
                assert!(true)
            } else {
                println!("{:?}", result2);
                assert!(false)
            }
        }


    }

    #[test]
    fn insert_struct_test() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();
        
        let result = tokio_test::block_on(client.insert(&TestStruct::new()));
        
        if let Ok(_) = result {
            assert!(true)
        } else {
            println!("{:?}", result);
            assert!(false)
        }

    }
    #[test]
    fn insert_batch_struct_test() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();
        let test_struct1 = TestStruct::new();
        let test_struct2 = TestStruct::new_min();
        let result = tokio_test::block_on(client.batch_insert(vec![&test_struct1, &test_struct2]));
        
        if let Ok(_) = result {
            assert!(true)
        } else {
            println!("{:?}", result);
            assert!(false)
        }

    }

    #[test]
    fn insert_struct_min_test() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();
        
        let result = tokio_test::block_on(client.insert(&TestStruct::new_min()));
        
        if let Ok(_) = result {
            assert!(true)
        } else {
            println!("{:?}", result);
            assert!(false)
        }

    }

    #[test]
    fn fetch_struct_test() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();
        
        let insert_result = tokio_test::block_on(client.insert(&TestStruct::new()));

        let fetch_result: Result<TestStruct, Box<dyn std::error::Error>> = tokio_test::block_on(client.fetch(insert_result.unwrap().remove("id").unwrap().parse::<u64>().unwrap()));
        
        if let Ok(record) = fetch_result {
            assert_eq!(record._char, 'c')
        } else {
            println!("{:?}", fetch_result);
            assert!(false)
        }
    }

    #[test]
    fn fetch_struct_parent_child_test() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();

        let parent_struct = TestOneToMany::new();
        
        let insert_result = tokio_test::block_on(client.insert(&parent_struct));
        
        let new_parent_id = insert_result.unwrap().remove("id").unwrap().parse::<u64>().unwrap();

        let mut test_many_to_one_collection = parent_struct.test_many_to_one_collection.unwrap();
        let mut child_iter = test_many_to_one_collection.iter_mut();

        while let Some(child) = child_iter.next() {
            child.test_one_to_many_id = Some(new_parent_id);
            let _child_id = tokio_test::block_on(client.insert(child));
        }

        let fetch_result: Result<TestOneToMany, Box<dyn std::error::Error>> = tokio_test::block_on(client.fetch(new_parent_id));
        
        if let Ok(record) = fetch_result {
            println!("{:?}", record);
            if let Some(vec) = record.test_many_to_one_collection {
                assert_eq!(vec.len(), 2)
            } else {
                println!("{:?}", record);
                assert!(false)
            }
        } else {
            println!("{:?}", fetch_result);
            assert!(false)
        }
    }

    #[test]
    fn update_basic_struct_test() {
        let options = DeviiClientOptions {
            login:  dotenv::var("DEVII_USERNAME").unwrap(),
            password: dotenv::var("DEVII_PASSWORD").unwrap(),
            tenantid:  dotenv::var("DEVII_TENANT_ID").unwrap().parse::<u32>().unwrap(),
            base:  dotenv::var("DEVII_BASE_URL").unwrap()
        };
        
        let client = tokio_test::block_on(DeviiClient::connect(options)).unwrap();
        
        let testing_struct = TestStruct::new();
        let mut testing_struct_dup = TestStruct::new();


        let insert_result = tokio_test::block_on(client.insert(&testing_struct));

        testing_struct_dup.id = Some(insert_result.unwrap().remove("id").unwrap().parse::<u64>().unwrap());

        let id_to_update = testing_struct_dup.id.clone().unwrap();

        testing_struct_dup.string = "I changed this".to_string();

        let update_result: Result<TestStruct, Box<dyn std::error::Error>> = tokio_test::block_on(client.update(testing_struct_dup, id_to_update));
        
        if let Ok(record) = update_result {
            assert_eq!(record.string, "I changed this".to_string());
        } else {
            println!("{:?}", update_result);
            assert!(false);
        }
    }
}
