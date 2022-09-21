use reqwest;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use std::collections::HashMap;

use blockchain_types::common::blockchain::{BlockChainStats, BlockChainNames, BlockChainStatType};


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
        let res = client.post(format!("{}/auth", options.base))
            .json(&options)
            .send()
            .await?
            .json::<DeviiClient>()
            .await?;

        Ok(res)
    }
    
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateChainStats {
    pub query: String,
    pub variables: String
}

impl GraphQLQuery for UpdateChainStats{}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateChainStatsVariables {
    id: u32,
    input: DeviiBlockChainStats
}

// Temporary until Devii fixes it's typing
#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiBlockChainStats {
    #[serde(skip_serializing)]
    id: Option<String>,
    pub blockchain_name: BlockChainNames,
    short_description: String,
    time_offset: f64, // seconds
    total_active_coins: f64,
    total_coin_issuance: f64,
    block_height: f64,
    active_addresses: f64,
    last_updated: f64,
    stat_type: BlockChainStatType,
    block_range_start: f64,
    block_range_end: f64,
    date_range_start: f64,
    date_range_end: f64,
}

pub trait DeviiQueryResultType{}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiQueryResult<T> {
    pub data: HashMap<String, T>
}

// cargo test foo -- --test-threads 3

#[cfg(test)]
mod tests {
    use dotenv;
    use crate::devii::DeviiClient;
    use crate::devii::DeviiClientOptions;

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
        }
    }
}
