use std::fs;
use std::time;
use web3::{
    contract::{Contract, Options},
    transports::WebSocket,
    types::{Address, U256},
    Web3,
};

struct ContractConfig<'a> {
    name: &'a str,
    symbol: &'a str,
    decimals: U256,
    initial_supply: U256,
}

async fn deploy_contract(
    config: &ContractConfig<'_>,
    web3: Web3<WebSocket>,
) -> web3::contract::Result<Address> {
    let accounts = web3.eth().accounts().await?;
    let bytecode = fs::read_to_string(format!("./vyper/target/{}.vy.bin", config.name)).unwrap();
    let bytecode = bytecode.trim_end().trim_start_matches("0x");
    let abi = fs::read(format!("./vyper/target/{}.vy.abi", config.name)).unwrap();
    let contract_builder = Contract::deploy(web3.eth(), &abi)?;
    let contract = contract_builder
        .confirmations(0)
        .poll_interval(time::Duration::from_millis(10))
        .options(Options::with(|opt| {
            opt.value = Some(5.into());
            opt.gas_price = Some(5.into());
            opt.gas = Some(3_000_000.into());
        }))
        .execute(
            bytecode,
            (
                config.name.to_string(),
                config.symbol.to_string(),
                config.decimals,
                config.initial_supply,
            ),
            accounts[0],
        )
        .await?;

    Ok(contract.address())
}

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let transport = web3::transports::WebSocket::new("ws://192.168.1.182:8545").await?;
    let web3 = web3::Web3::new(transport);
    let contracts: Vec<ContractConfig> = vec![
        ContractConfig {
            name: "berserkers",
            symbol: "SQB",
            decimals: U256::from(18 as u64),
            initial_supply: U256::from(0u64),
        },
        ContractConfig {
            name: "eggs",
            symbol: "SQE",
            decimals: U256::from(18 as u64),
            initial_supply: U256::from(0u64),
        },
        ContractConfig {
            name: "guardians",
            symbol: "SQG",
            decimals: U256::from(18 as u64),
            initial_supply: U256::from(0u64),
        },
        ContractConfig {
            name: "queens",
            symbol: "SQQ",
            decimals: U256::from(18 as u64),
            initial_supply: U256::from(0u64),
        },
        ContractConfig {
            name: "sacred_queens",
            symbol: "SQSQ",
            decimals: U256::from(18 as u64),
            initial_supply: U256::from(1_000_000u64)
                .checked_mul(U256::exp10(18))
                .unwrap(),
        },
    ];
    for c in contracts {
        let address = deploy_contract(&c, web3.clone()).await?;
        println!("Contract {}.vy deployed at: {:?}", c.name, address);
    }
    Ok(())
}
