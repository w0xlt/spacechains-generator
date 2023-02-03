use std::str::FromStr;

use bdk::bitcoin::{self, Network};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub network: String,
    pub electrum_url: String,
    pub receiving_desc: String,
    pub change_desc: String,
    pub dust_limit: u64,
    pub fee_amount: u64,
}

impl ConfigFile {
    #[allow(dead_code)]
    pub fn get_network(&self) -> Result<Network, std::string::String> {
        if self.network == "signet" {
            return Ok(bitcoin::Network::Signet);
        } else if self.network == "testnet" {
            return Ok(bitcoin::Network::Testnet);
        }
        Err("Only signet supported for now".to_string())
    }
}

pub fn create_or_get_default() -> (ConfigFile, String) {
    let home_dir = dirs::home_dir();

    let mut path = home_dir.clone().unwrap();
    path.push(".spacechains");

    std::fs::create_dir_all(path.clone()).unwrap();

    path.push("generator.conf");

    let binding = path.clone();
    let path_str = binding.as_os_str().to_str().unwrap();

    let mut bc_path = home_dir.unwrap();
    bc_path.push(".bitcoin");
    bc_path.push("signet");
    bc_path.push(".cookie");

    if !path.exists() {
        let cfg = ConfigFile {
            network: String::from_str("signet").unwrap(),
            electrum_url: String::from_str("tcp://127.0.0.1:50001").unwrap(),
            receiving_desc: String::from_str("").unwrap(),
            change_desc: String::from_str("").unwrap(),
            dust_limit: 800,
            fee_amount: 1200,
        };

        confy::store_path(path, &cfg).unwrap();

        (cfg, String::from_str(path_str).unwrap())
    } else {
        let cfg: ConfigFile = confy::load_path(path).unwrap();

        (cfg, String::from_str(path_str).unwrap())
    }
}
