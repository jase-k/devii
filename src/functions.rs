use blockchain_types::common::blockchain::{BlockChainStats};
use crate::devii::{DeviiBlockChainStats, DeviiQueryResult, DeviiQueryOptions, DeviiClient, DeviiClientOptions, UpdateChainStats};
use dotenv;


async fn connect_to_db() -> Result<DeviiClient, Box<dyn std::error::Error>> {
    #[allow(non_snake_case)]
    let DEVII_USERNAME: String = dotenv::var("DEVII_USERNAME").unwrap();
    #[allow(non_snake_case)]
    let DEVII_PASSWORD: String = dotenv::var("DEVII_PASSWORD").unwrap();

    let devii_options = DeviiClientOptions::new(DEVII_USERNAME,DEVII_PASSWORD);

    let client = DeviiClient::connect(devii_options).await?;

    Ok(client)
}

// TODO add filter for different blockchain types
pub async fn get_default_blockchain_stats() -> Result<Vec<BlockChainStats>, Box<dyn std::error::Error>> {
    let client = connect_to_db().await?;

    let query = DeviiQueryOptions {
        query: std::fs::read_to_string("./src/utils/queries/chain_stats.graphql").unwrap()
    };
    println!("{:?}", query);
    
    let result = client
        .query::<DeviiQueryResult<Vec<DeviiBlockChainStats>>, DeviiQueryOptions>(query).await?;

    let devii_stat_types: &Vec<DeviiBlockChainStats> = result.data.get("chain_stats").unwrap();
    let mut devii_stat_types_iter = devii_stat_types.iter();

    let mut result_vec: Vec<BlockChainStats> = vec![];

    while let Some(devii_stat) = devii_stat_types_iter.next() {
        result_vec.push(devii_stat.to_blockchain_stat())
    }
    
    Ok(result_vec)
}

pub async fn update_default_blockchain_stats(stat: &mut BlockChainStats) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_to_db().await?;

    let query = UpdateChainStats {
        query: std::fs::read_to_string("./src/utils/queries/update_chain_stats.graphql").unwrap(),
        variables: format!("{{ 
            \"id\" : {}, 
            \"input\" : {{
                \"total_coin_issuance\": {},
                \"blockchain_name\": \"{}\",
                \"short_description\": \"{}\",
                \"active_addresses\": {},
                \"block_height\": {},
                \"last_updated\": {},
                \"stat_type\": \"{}\",
                \"time_offset\": {},
                \"total_active_coins\": {},
                \"date_range_start\": {},
                \"date_range_end\": {},
                \"block_range_start\": {},
                \"block_range_end\": {}
            }}
        }}", 
            stat.id(),
            stat.total_coin_issuance(), 
            stat.blockchain_name(),
            stat.short_description(),
            stat.active_address_total(),
            stat.block_height(),
            stat.last_updated(),
            stat.stat_type(),
            stat.time_offset(),
            stat.total_coin_in_circulation(),
            // Need to change below to edit the actual range. 
            stat.date_range_start(),
            stat.date_range_end(),
            stat.block_range_start(),
            stat.block_range_end()
        )
    };
    println!("{:?}", query);
    
    let _result = client
        .query::<DeviiQueryResult<DeviiBlockChainStats>, UpdateChainStats>(query).await?;

    Ok(())
}