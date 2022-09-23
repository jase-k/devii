use reqwest;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use named_type::NamedType;
use convert_case::{Case, Casing};


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
    pub query: String
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
pub struct Insert<T: Serialize> {
    // Docs: https://serde.rs/attr-bound.html
    #[serde(bound(deserialize = "T: Deserialize<'de>"))]
    input: T
}

impl <T: DeserializeOwned + Serialize>GraphQLQuery for DeviiQueryInsertOptions<T>{}


impl DeviiClient {
    pub async fn connect(options: DeviiClientOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        // println!("Connection Result {:?}", client.post(format!("{}/auth", options.base))
        // .json(&options)
        // .send()
        // .await?
        // .text()
        // .await?);

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

        // println!("Request: {:?}", client.post(&self.routes.query)
        // .header("Authorization", format!("Bearer {}", self.access_token))
        // .json(&options)
        // .send()
        // .await?
        // .text().await);
        
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

// cargo test foo -- --test-threads 3

#[cfg(test)]
mod tests {
    use dotenv;
    use crate::devii::DeviiClient;
    use crate::devii::DeviiClientOptions;
    use crate::test_struct::TestStruct;

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
}
