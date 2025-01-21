use clap::{Parser, Subcommand};
use toner::contracts::wallet::KeyPair;
use toner::contracts::wallet::mnemonic::Mnemonic;
use tonlib::cell::TonCellError;
use tonlib::client::{TonClient, TonClientBuilder, TonClientInterface, TonConnectionParams};
use tonlib::tl::{BlocksShards};
use tonlib::wallet::{TonWallet, WalletVersion};
use dialoguer::{theme::ColorfulTheme, Select};
use inline_colorization::{color_bright_green, color_green, color_red, color_reset, color_yellow};
use spinners::{Spinner, Spinners};

pub const TESTNET_CONFIG: &str = include_str!("../testnet-global.config.json");

/// CLI app for generate a TON account with a specific shard
#[derive(Parser)]
#[command(name = "ton-cli", version = "1.0", about = "TON Blockchain CLI Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new wallet with assigned shard
    Generate {
        /// Specify the shard to assign to the account (choose from predefined options)
        #[arg(long)]
        shard: Option<String>,
    },
    /// Detect the shard for a given address
    Shard {
        /// Address to check the shard
        address: String,
    },
}


/// Validate shard input against predefined options
fn validate_shard(net_shards: Vec<u64>, shard: u64) -> Result<(), String> {
    if net_shards.contains(&shard) {
        Ok(())
    } else {
        Err(format!(
            "Invalid shard. Choose from: {}",
            net_shards.iter().map(|num| format!("{:x}", num)).collect::<Vec<String>>().join(", ")
        ))
    }
}

/// Generate a new mnemonic
fn generate_key_pair() -> (KeyPair, String) {
    let bip_mnem = bip39::Mnemonic::generate(24).unwrap();
    //todo: cannot convert bip39 mnemonic to tonlib mnemonic directly
    let ton_contract_mnem: Mnemonic = bip_mnem.to_string().parse().unwrap();
    let kp: KeyPair = ton_contract_mnem.generate_keypair(None).unwrap();

    (kp, bip_mnem.to_string())
}

/// Create an account using a mnemonic
fn export_wallet_from_key_pair(key_pair: KeyPair) -> Result<TonWallet, TonCellError> {
    // Convert the ton-contract KeyPair to tonlib KeyPair
    let key_pair = tonlib::mnemonic::KeyPair {
        public_key: Vec::from(key_pair.public_key),
        secret_key: Vec::from(key_pair.secret_key),
    };
    //let wallet = TonWallet::derive(0, WalletVersion::V4R2, &key_pair, 1);
    TonWallet::derive_default(WalletVersion::V4R2, &key_pair)
}


async fn get_shards_from_network() ->  anyhow::Result<(TonClient, Vec<u64>)> {
    TonClient::set_log_verbosity_level(0);
    let client = TonClientBuilder::new()
        .with_pool_size(10)
        .with_connection_params(&TonConnectionParams{
            config: TESTNET_CONFIG.to_string(),
            ..Default::default()
        })
        .build()
        .await?;

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
fn get_shard(net_shards: &Vec<u64>, account_id: &str) -> Option<u64> {
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

#[tokio::main]
async fn main() {
    println!("Welcome TON Shard master tool.");
    println!();
    let cli = Cli::parse();
    let (_client, net_shards) = get_shards_from_network().await.unwrap();
    let hex_string = net_shards
        .iter()
        .map(|num| format!("{:x}", num)) // Convert each i64 to hex
        .collect::<Vec<String>>(); // Collect into a Vec of hex strings ; // Join them with a separator (optional)
    println!("Network shards are available (hex): {:?}", hex_string.join(", "));


    match cli.command {
        Commands::Generate { shard } => {
            let start_time = std::time::Instant::now();

            let user_shard = match shard {
                Some(shard) => shard,
                None => {
                    let shard_id = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Choose a shard for the wallet:")
                        .items(&hex_string)
                        .interact()
                        .unwrap();
                    hex_string[shard_id].clone()
                },
            };

            let shard = user_shard.to_lowercase();
            println!("Assigned Shard (hex): {}", shard);

            let shard = u64::from_str_radix(&shard, 16).unwrap();
            if let Err(err) = validate_shard(net_shards.clone(), shard) {
                eprintln!("{}", err);
                return;
            }

            let mut sp = Spinner::new(Spinners::CircleHalves, "".to_string());

            loop {
                let (key_pair, mnemonic_string) = generate_key_pair();
                let wallet = export_wallet_from_key_pair(key_pair).unwrap();


                let maby_account_shard =  get_shard(&net_shards, wallet.address.to_hex().as_str());
                if let Some(account_shard) = maby_account_shard {
                    if account_shard == shard {
                        println!();
                        println!("Wallet address: {color_yellow}{:?}{color_reset}", wallet.address);
                        println!("Wallet address(HEX): {color_yellow}{:?}{color_reset}", wallet.address.to_hex());
                        println!("{color_green}Shard is FOUND <:). account_shard: {:x?}, expected: {:x?}{color_reset}", account_shard, shard);
                        println!("Account mnemonic: {color_bright_green}{:?}{color_reset}", mnemonic_string);
                        sp.stop_with_newline();
                        break;
                    } else {
                        println!("{color_red}Shard is not equal to assigned shard, got: {:x?}, expect: {:x?}{color_reset}", account_shard, shard);
                    }

                } else {
                    println!("Shard is not found");
                }
            }

            println!("Elapsed time: {:?}", start_time.elapsed());

        }
        Commands::Shard { address } => {
            match get_shard(&net_shards, address.as_str()) {
                Some(shard) => println!("Shard: {}", shard),
                None => println!("Shard: Not found"),
            }
        }
    }
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
        ];

        for (account_id, expect_shard) in addresses_shard {
            let got_shard = get_shard(&net_shards, account_id);
            assert_eq!(got_shard, Some(expect_shard), "shard must be equal, but got: {:x?}, expect: {:x?}, address: {:?}", got_shard, expect_shard, account_id);
        }
    }
}