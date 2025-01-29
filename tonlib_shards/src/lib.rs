use tonlib::client::{TonClient, TonClientInterface};
use tonlib::tl::BlocksShards;

/// Get the list of shards from the network
pub async fn get_shards_from_network(client: TonClient) ->  anyhow::Result<(TonClient, Vec<u64>)> {
    let (_, info) = client.get_masterchain_info().await?;
    let block_shards: BlocksShards = client.get_block_shards(&info.last).await?;
    let mut shards = block_shards.shards.clone();

    shards.insert(0, info.last.clone());
    let mut net_shards = Vec::new();
    for shard in shards {
        // skip masterchain shard 8000000000000000
        if shard.shard as u64 == 9223372036854775808u64 {
            continue;
        }
        net_shards.push(shard.shard as u64);
    }

    Ok((client, net_shards))
}


/// Extract the top 64 bits from a 256-bit account ID
fn extract_top64(account_id: &str) -> Option<u64> {
    // Remove the `0:` prefix if present and parse the hex string into bytes
    let hex_part = account_id.trim_start_matches("0:");
    let bytes = hex::decode(hex_part).ok()?;

    // Ensure we have exactly 32 bytes (256 bits)
    if bytes.len() != 32 {
        return None;
    }

    // Extract the first 8 bytes (top 64 bits) as a u64
    Some(u64::from_be_bytes(bytes[0..8].try_into().unwrap()))
}

/// Get the shard for a given account ID
pub fn get_shard(net_shards: &Vec<u64>, account_id: &str) -> Option<u64> {
    if let Some(top64) = extract_top64(account_id) {
        for &shard in net_shards {
            if shard_contains(shard, top64) {
                return Some(shard);
            }
        }
    }
    None
}

/// Check if a shard contains the given account prefix
fn shard_contains(shard: u64, account_prefix: u64) -> bool {
    let x = shard.trailing_zeros();
    let mask = (!0u64) << (x + 1);
    (shard ^ account_prefix) & mask == 0
}


#[cfg(test)]
mod tests {
    use super::*;
    const SHARDS: [u64; 4] = [
        0x2000000000000000,
        0x6000000000000000,
        0xA000000000000000,
        0xE000000000000000,
    ];
    #[test]
    fn it_works() {

        let net_shards = SHARDS.to_vec();

        let addresses_shard = [
            ("0:af78316b56ee5f7e88f3558ad3b5ebbafd49304249e48dd33c9f27e63b7c8fe7",0xa000000000000000),
            ("0:80fa1ebdd70277ca902d52cb2007cf910ca572b80f7c186fbb86e116cf4c66ba",0xa000000000000000),
            ("0:923150e0c668cb309dc3d43449be197e17f5095378260e7715e278eaa80941ab",0xa000000000000000),
            ("0:684c17d1138bcd4355aa88cc30dacba8cda4d8f3de4392cb5a7f4bec030190af",0x6000000000000000),
            ("0:51cca3ff74207b3ed8f075740b126c320e795ec4f19f70b80d9cf919fc292594",0x6000000000000000),
            ("0:b19a8a1821d01279aeb98e84a2ed002e4a30633264702b1059cebe73100d6b95",0xa000000000000000),
        ];

        for (account_id, expect_shard) in addresses_shard {
            let got_shard = get_shard(&net_shards, account_id);
            assert_eq!(got_shard, Some(expect_shard), "shard must be equal, but got: {:x?}, expect: {:x?}, address: {:?}", got_shard, expect_shard, account_id);
        }
    }
}