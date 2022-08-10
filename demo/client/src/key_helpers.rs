use {anyhow::Result, ed25519_dalek::*, rand::rngs::OsRng};

pub fn get_keypair(privkey: String) -> Result<Keypair> {
    let decoded_privkey: &[u8] = &bs58::decode(privkey).into_vec()?;
    let k = Keypair::from_bytes(decoded_privkey)?;
    Ok(k)
}

pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng {};
    Keypair::generate(&mut csprng)
}

pub fn get_pubkey(keypair: &Keypair) -> String {
    bs58::encode(keypair.public.to_bytes()).into_string()
}

pub fn get_privkey(keypair: &Keypair) -> String {
    bs58::encode(keypair.to_bytes()).into_string()
}
