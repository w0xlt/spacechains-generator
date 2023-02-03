use std::collections::HashMap;

use bdk::{
    bitcoin::{
        blockdata::{opcodes, script},
        hashes::hex::ToHex,
        psbt::{self, PartiallySignedTransaction},
        secp256k1::{rand::thread_rng, PublicKey, Secp256k1, SecretKey},
        OutPoint, PrivateKey, Script, Transaction, Txid,
    },
    database::MemoryDatabase,
    wallet::AddressIndex,
    Error, KeychainKind, SignOptions, Wallet,
};

use crate::config_file::ConfigFile;

fn generate_ephemeral_covenant_descriptor(cfg: &ConfigFile) -> (String, String) {
    let network = cfg.get_network().unwrap();

    let sk = SecretKey::new(&mut thread_rng());
    let private_key = PrivateKey::new(sk, network);

    let wif_key = PrivateKey::to_wif(private_key);

    let curve = Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&curve, &sk);
    let public_key_hex = public_key.serialize().to_hex();

    let private_desc = format!("wsh(and_v(v:pk({}),older(1)))", wif_key);
    let public_desc = format!("wsh(and_v(v:pk({}),older(1)))", public_key_hex);

    (private_desc, public_desc)
}

pub fn load_convenant_wallet(cfg: &ConfigFile) -> (Wallet<MemoryDatabase>, String) {
    let (private_desc, public_desc) = generate_ephemeral_covenant_descriptor(cfg);

    let network = cfg.get_network().unwrap();

    let convenant_wallet = Wallet::new(
        private_desc.as_str(),
        None,
        network,
        MemoryDatabase::default(),
    )
    .unwrap();

    (convenant_wallet, public_desc)
}

pub fn generate_sequential_convenant_transactions(
    cfg: &ConfigFile,
    convenant_wallet: &Wallet<MemoryDatabase>,
    transaction: &Transaction,
) -> HashMap<Txid, Transaction> {
    let mut tx_map: HashMap<Txid, Transaction> = HashMap::new();

    let mut tx = transaction.clone();

    let satisfaction_weight = convenant_wallet
        .get_descriptor_for_keychain(KeychainKind::External)
        .max_satisfaction_weight()
        .unwrap();

    let (dust_limit, fee_amount) = (cfg.dust_limit, cfg.fee_amount);

    let mut count: u32 = 0;
    loop {
        let previous_txid = tx.clone().txid();
        let opt_tx = build_squential_transaction(
            &convenant_wallet,
            &tx,
            dust_limit,
            fee_amount,
            satisfaction_weight,
        );
        if opt_tx == None {
            break;
        }
        tx = opt_tx.unwrap();
        println!("tx.output[0].value {}", tx.output[0].value);
        println!("tx.output[1].value {}", tx.output[1].value);
        tx_map.insert(previous_txid, tx.clone());
        count = count + 1;
        println!("count: {}", count);
    }

    tx_map
}

fn build_bump_script(add_op_3: bool) -> Script {
    let mut builder = script::Builder::new();

    if add_op_3 {
        builder = builder.push_opcode(opcodes::all::OP_PUSHBYTES_3);
    }

    builder
        .push_opcode(opcodes::all::OP_PUSHBYTES_0)
        .push_opcode(opcodes::all::OP_CSV)
        .push_opcode(opcodes::all::OP_1ADD)
        .into_script()
}

fn build_squential_transaction(
    convenant_wallet: &Wallet<MemoryDatabase>,
    previous_tx: &Transaction,
    dust_limit: u64,
    fee_amount: u64,
    satisfaction_weight: usize,
) -> Option<Transaction> {
    let convenant_address = convenant_wallet
        .get_address(AddressIndex::New)
        .unwrap()
        .address;

    let mut tx_builder = convenant_wallet.build_tx();

    let mut vout: u32 = 0;

    if previous_tx.output[0].script_pubkey != convenant_address.script_pubkey() {
        assert!(previous_tx.output[1].script_pubkey == convenant_address.script_pubkey());
        vout = 1;
    }

    let input_satoshis = previous_tx.output[vout as usize].value;

    if input_satoshis < dust_limit + fee_amount {
        return None;
    }

    let outpoint = OutPoint {
        txid: previous_tx.txid(),
        vout,
    };

    let psbt_input = psbt::Input {
        non_witness_utxo: Some(previous_tx.clone()),
        witness_utxo: Some(previous_tx.output[vout as usize].clone()),
        ..Default::default()
    };

    tx_builder
        .add_foreign_utxo(outpoint, psbt_input, satisfaction_weight)
        .unwrap();

    tx_builder.manually_selected_only();

    tx_builder.current_height(0);

    tx_builder.fee_absolute(fee_amount);

    tx_builder.version(2);

    let bump_script = build_bump_script(false);

    let covenant_amount = input_satoshis - dust_limit - fee_amount;

    let bump_amount = dust_limit;

    tx_builder.set_recipients(vec![
        (bump_script.to_v0_p2wsh(), bump_amount),
        (convenant_address.script_pubkey(), covenant_amount),
    ]);

    let finished: Option<PartiallySignedTransaction> = match tx_builder.finish() {
        Ok(result) => Some(result.0),
        Err(err) => {
            if matches!(err, Error::OutputBelowDustLimit(_)) {
                return None;
            } else {
                panic!("{}", err);
            }
        }
    };

    match finished {
        Some(mut psbt) => {
            convenant_wallet
                .sign(&mut psbt, SignOptions::default())
                .unwrap();
            Some(psbt.extract_tx())
        }
        None => None,
    }
}
