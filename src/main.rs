use std::{collections::HashMap, io};

use bdk::{
    bitcoin::{consensus::serialize, Transaction, Txid},
    wallet::AddressIndex,
};
use rusqlite::{params, Connection};

mod config_file;
mod covenant;
mod utils;
mod wallet_manager;

fn main() {
    let (cfg, cfg_path) = config_file::create_or_get_default();

    if cfg.change_desc.is_empty() || cfg.receiving_desc.is_empty() {
        println!("receiving_desc and change_desc in the configuration file must be filled to initialize the wallet.");
        println!("The config file is in {}", cfg_path);
        return;
    }

    let funding_wallet = wallet_manager::load_wallet(&cfg);

    utils::sync_wallet(&cfg, &funding_wallet);

    let (convenant_wallet, public_desc) = covenant::load_convenant_wallet(&cfg);

    let convenant_address = convenant_wallet
        .get_address(AddressIndex::New)
        .unwrap()
        .address;

    let args: Vec<String> = std::env::args().collect();

    let amount = args[1].parse::<u64>().unwrap();

    let funding_tx = wallet_manager::create_genesis_covenant_transaction(
        &funding_wallet,
        &convenant_address,
        amount,
    );

    let tx_map =
        covenant::generate_sequential_convenant_transactions(&cfg, &convenant_wallet, &funding_tx);

    let positive_response = vec!["y", "Yes", "YES", "yes", "Y"];
    let negative_response = vec!["n", "No", "NO", "no", "N"];

    let mut input_string = String::new();
    while !positive_response.contains(&input_string.trim())
        && !negative_response.contains(&input_string.trim())
    {
        println!(
            "This will generated {} pre-signed transactions",
            tx_map.len()
        );
        println!("Do you want to continue? (y/n):");
        input_string.clear();
        io::stdin().read_line(&mut input_string).unwrap();
    }

    if negative_response.contains(&input_string.trim()) {
        return;
    }

    utils::broadcast_tx(&cfg, &funding_tx);

    println!("funding_tx_id: {}", funding_tx.txid());

    write_db(&tx_map, &public_desc);

    println!("Success ! File convenant.db with pre-signed transactions created. It can be included in the spacehain project.");
    println!(
        "The funding transaction {} needs to be confirmed before the pre-signed ones be used.",
        funding_tx.txid()
    );
    println!("Otherwise, it will generate non-BIP68-final error due to older(1) condition, which means OP_PUSHNUM_1 OP_CSV in the script.");
}

pub fn write_db(tx_map: &HashMap<Txid, Transaction>, public_desc: &String) {
    let conn = Connection::open("convenant.db").unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS convenant_txs (previous_tx_id BLOB UNIQUE, tx_hex BLOB);",
        [],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS convenant_descriptor (public_descriptor TEXT);",
        [],
    )
    .unwrap();

    for (prev_txid, tx) in tx_map {
        let tx_bytes = serialize(tx);

        let prev_txid_bytes = serialize(prev_txid);

        conn.execute(
            "INSERT INTO convenant_txs (previous_tx_id, tx_hex) VALUES (?1, ?2)",
            params![prev_txid_bytes, tx_bytes],
        )
        .unwrap();
    }

    conn.execute(
        "INSERT INTO convenant_descriptor (public_descriptor) VALUES (?1)",
        params![public_desc],
    )
    .unwrap();
}
