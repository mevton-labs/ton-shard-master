use std::str::FromStr;
use clap::{Parser, Subcommand};
use tonlib::cell::{TonCellError};
use tonlib::client::{TonClient, TonClientBuilder, TonConnectionParams};
use tonlib::wallet::{TonWallet, WalletVersion};
use dialoguer::{theme::ColorfulTheme, Select};
use inline_colorization::{color_bright_green, color_green, color_red, color_reset, color_yellow};
use spinners::{Spinner, Spinners};
use tonlib::address::TonAddress;
use tonlib::mnemonic::{KeyPair, Mnemonic};
use tonlib_shards::{get_shard, get_shards_from_network};

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
    let mut bip_mnem;
    let tonlib_mnem ;
    loop {
        bip_mnem = bip39::Mnemonic::generate(24).unwrap();
        tonlib_mnem = match Mnemonic::from_str(&bip_mnem.to_string(), &None) {
            Ok(mnem) => {mnem},
            Err(_) => {
                continue
            },
        };
        break;
    }

    let kp: KeyPair = tonlib_mnem.to_key_pair().unwrap();

    (kp, bip_mnem.to_string())
}

/// Create an account using a mnemonic
fn export_wallet_from_key_pair(key_pair: KeyPair) -> Result<TonWallet, TonCellError> {
    //let wallet = TonWallet::derive(0, WalletVersion::V4R2, &key_pair, 1);
    TonWallet::derive_default(WalletVersion::V4R2, &key_pair)
}


#[tokio::main]
async fn main() {
    println!("Welcome TON Shard master tool.");
    println!();
    let cli = Cli::parse();
    TonClient::set_log_verbosity_level(0);
    let ton_client = TonClientBuilder::new()
        .with_pool_size(10)
        .with_connection_params(&TonConnectionParams{
            config: TESTNET_CONFIG.to_string(),
            ..Default::default()
        })
        .build()
        .await.expect("Failed to create TonClient");
    let (_client, net_shards) = get_shards_from_network(ton_client).await.unwrap();
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
                        println!("Save this information for later use:");
                        println!("{color_green}Shard is FOUND <:). account_shard: {:x?}, expected: {:x?}{color_reset}", account_shard, shard);
                        println!("Wallet address: {color_yellow}{:?}{color_reset}", wallet.address);
                        println!("Wallet address(HEX): {color_yellow}{:?}{color_reset}", wallet.address.to_hex());
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
            let ton_address = TonAddress::from_str(&address).unwrap();
            match get_shard(&net_shards, ton_address.to_hex().as_str()) {
                Some(shard) => println!("Shard: {:x?}", shard),
                None => println!("Shard: Not found"),
            }
        }
    }
}
