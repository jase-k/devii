use reqwest;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use named_type::NamedType;
use convert_case::{Case, Casing};
use struct_field_names_as_array::FieldNamesAsArray;
use core::fmt::Debug;
use std::fmt::Display;
use serde_json::{Map, Value};
use easy_error::bail;

pub trait GraphQLQuery{}


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
    pub fn new(login: String, password: String, base: String) -> Self {
        DeviiClientOptions {
            login: login,
            tenantid: 13,
            password: password,
            base: base
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

pub trait FieldNamesAsArray {
    fn fields(&self) -> String {
        stringify!((self::FIELD_NAMES_AS_ARRAY)).to_string()
    }
}


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
    pub async fn query<T: DeserializeOwned, K : GraphQLQuery + Serialize>(&self, options: K) -> Result<T, Box< dyn std::error::Error>>
    {
        let client = reqwest::Client::new();
        //Add Auth header
        let res = client.post(&self.routes.query)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&options)
            .build()?;
        let execute_result = client.execute(res)
        .await?;

        println!("Request: {:?}", client.post(&self.routes.query)
        .header("Authorization", format!("Bearer {}", self.access_token))
        .json(&options)
        .send()
        .await?
        .text().await);
        
        let result = execute_result    
            .json::<T>()
            .await?;

        Ok(result)
    }

    // returns id -> BigSerial Type needed in Postgres column
    pub async fn insert<T: DeserializeOwned + Serialize + NamedType>(&self, object: T) -> Result<u64, Box<dyn std::error::Error>> {
        // create query. 
        let insert = Insert {
            input : object
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

        let mut result = self.query::<DeviiQueryResult<InsertIdResult>, DeviiQueryInsertOptions<T>>(query).await?;

        let id_from_insert = result.data.remove(&(format!("create_{}", snake_type))).unwrap();
        
        Ok(id_from_insert.id.parse::<u64>().unwrap())
    }

    pub async fn fetch<T: DeserializeOwned + Serialize + NamedType + Default>(&self, id: u64) -> Result<T, Box<dyn std::error::Error>> {
        let snake_type = T::short_type_name().to_case(Case::Snake);

        let query_string = format!("query fetch($filter: String){{
            {} (filter: $filter)
              {}
            
          }}",
          snake_type,
          parse_value(&serde_json::to_value(T::default()).unwrap()) 
        );
        let fetch_variables = FetchOptionsBuilder::default().filter(format!("id = {}", id)).build().unwrap();
        
        let query = DeviiQueryOptions{ 
            query: query_string,
            variables: Some(fetch_variables)
        };

        let mut result = self.query::<DeviiQueryResult<Vec<T>>, DeviiQueryOptions>(query).await?;

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
          parse_value(&serde_json::to_value(T::default()).unwrap())
        );

        let query = DeviiQueryUpdateOptions{ 
            query: query_string,
            variables: update
        };

        let mut result = self.query::<DeviiQueryResult<T>, DeviiQueryUpdateOptions<T>>(query).await?;

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
    // Result comes back from devii as a string but should be a bigserial number
    id: String
}

fn parse_value(value: &Value) -> String {
    match value {
        Value::Object(map) => { return ["{", parse_object(&map).as_str(), "}"].join("") },
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
                map_vals.push(parse_value(value));
            }
        }
    map_vals.join("")
}

// TODO: Currently #[serde(skip_serializing_if = "example")] is not supported as this may result in not all the vec params being found.
// Also doesn't work for empty string... 
fn parse_array(vec: &Vec<Value> ) -> String {
    let vec_obj = vec.iter().next();
    if let Some(o) = vec_obj {
        return parse_value(o);
    } else {
        return "".to_string();
    }
}


// cargo test foo -- --test-threads 3
#[cfg(test)]
mod tests {
    use dotenv;
    use crate::devii::DeviiClient;
    use crate::devii::DeviiClientOptions;
    use crate::test_struct::TestStruct;
    use crate::devii::parse_value;

    #[test]
    fn parse_value_test() {
        let mut value = serde_json::to_value(TestStruct::default()).unwrap();
        let result = parse_value(&value);
        assert_eq!("{_char,_f32,_f64,_i16,_i32,_i64,_i8,_u16,_u32,_u8,string}".to_string()
        , result)
    }
    // May be flaky? 
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
            assert_eq!(res.routes.query, format!("{}/jase/query",dotenv::var("DEVII_BASE_URL").unwrap()));
        } else {
            assert!(false);
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
        
        let result = tokio_test::block_on(client.insert(TestStruct::new()));
        
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
        
        let result = tokio_test::block_on(client.insert(TestStruct::new_min()));
        
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
        
        let insert_result = tokio_test::block_on(client.insert(TestStruct::new()));

        let fetch_result: Result<TestStruct, Box<dyn std::error::Error>> = tokio_test::block_on(client.fetch(insert_result.unwrap()));
        
        if let Ok(record) = fetch_result {
            assert_eq!(record._char, 'c')
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


        let insert_result = tokio_test::block_on(client.insert(testing_struct));

        testing_struct_dup.id = Some(insert_result.unwrap());

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
