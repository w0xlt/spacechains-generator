use bdk::{
    bitcoin::Transaction,
    blockchain::{Blockchain, ElectrumBlockchain},
    database::MemoryDatabase,
    electrum_client::Client,
    SyncOptions, Wallet,
};

use crate::config_file::ConfigFile;

pub fn broadcast_tx(cfg: &ConfigFile, transaction: &Transaction) {
    let electrum_url = &cfg.electrum_url;

    let blockchain = ElectrumBlockchain::from(Client::new(electrum_url).unwrap());

    blockchain.broadcast(transaction).unwrap();
}

pub fn sync_wallet(cfg: &ConfigFile, wallet: &Wallet<MemoryDatabase>) {
    let electrum_url = &cfg.electrum_url;

    let blockchain = ElectrumBlockchain::from(Client::new(electrum_url).unwrap());

    wallet.sync(&blockchain, SyncOptions::default()).unwrap();
}
