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
    password: String
}

impl DeviiClientOptions {
    pub fn new(login: String, password: String) -> Self {
        DeviiClientOptions {
            login: login,
            tenantid: 13,
            password: password
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiQueryOptions {
    pub query: String
}

impl GraphQLQuery for DeviiQueryOptions{}

#[derive(Serialize, Debug)]
pub struct DeviiQueryInsertOptions<T: Serialize + DeserializeOwned> {
    pub query: String,
    pub variables: Insert<T>
}
impl <T: Serialize + DeserializeOwned>DeserializeOwned for DeviiQueryInsertOptions<T>{}
impl <T: Serialize + DeserializeOwned>DeserializeOwned for Insert<T>{}

#[derive(Serialize, Debug)]
pub struct Insert<T: Serialize + DeserializeOwned> {
    input: T
}

impl <T>GraphQLQuery for DeviiQueryInsertOptions<T>{}


impl DeviiClient {
    pub async fn connect(options: DeviiClientOptions) -> Result<Self, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let res = client.post("https://devii-experimental.centralus.cloudapp.azure.com/auth")
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

// impl DeviiBlockChainStats {
//     pub fn to_blockchain_stat(&self) -> BlockChainStats {
//         BlockChainStats::new(
//             self.blockchain_name.clone(),
//             self.id.clone().unwrap(),
//             self.short_description.clone(),
//             self.time_offset as u32, // seconds
//             self.total_active_coins,
//             self.total_coin_issuance,
//             self.block_height as u32,
//             self.active_addresses as u32,
//             self.last_updated as u32,
//             self.stat_type.clone(),
//             self.block_range_start as u32,
//             self.block_range_end as u32,
//             self.date_range_start as u32,
//             self.date_range_end as u32,
//         )
//     }
// }

pub trait DeviiQueryResultType{}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeviiQueryResult<T> {
    pub data: HashMap<String, T>
}