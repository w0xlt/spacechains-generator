use bdk::{
    bitcoin::{Address, Transaction},
    database::MemoryDatabase,
    SignOptions, Wallet,
};

use crate::config_file::ConfigFile;

pub fn load_wallet(cfg: &ConfigFile) -> Wallet<MemoryDatabase> {
    let network = cfg.get_network().unwrap();

    Wallet::new(
        cfg.receiving_desc.as_str(),
        Some(cfg.change_desc.as_str()),
        network,
        MemoryDatabase::default(),
    )
    .unwrap()
}

pub fn create_genesis_covenant_transaction(
    wallet: &Wallet<MemoryDatabase>,
    convenant_address: &Address,
    amount: u64,
) -> Transaction {
    let mut tx_builder = wallet.build_tx();

    tx_builder.add_recipient(convenant_address.script_pubkey(), amount);

    let (mut psbt, _) = tx_builder.finish().unwrap();

    wallet.sign(&mut psbt, SignOptions::default()).unwrap();

    psbt.extract_tx()
}
