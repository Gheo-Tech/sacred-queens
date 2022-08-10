use {
    ed25519_dalek::*,
    futures::stream::TryStreamExt,
    mongodb::{
        bson::doc, error::Error as MongoError, options::FindOptions, options::IndexOptions, Client,
        ClientSession, Collection, Database, IndexModel,
    },
    rand::rngs::OsRng,
    serde::{de::DeserializeOwned, Deserialize, Serialize},
    std::sync::Arc,
};

pub const SWARMS_COLL_NAME: &str = "swarms";
pub const SACRED_HIVE_COLL_NAME: &str = "sacredHives";
pub const HIVE_COLL_NAME: &str = "hives";

#[derive(Clone, Deserialize, Serialize)]
pub struct Swarm {
    pub pubkey: String,
    pub sacred_queens: i64,
    pub queens: i64,
    pub guardians: i64,
    pub berserkers: i64,
    pub eggs: i64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SacredHive {
    pub pubkey: String,
    pub sacred_queens: i64,
    pub eggs: i64,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Hive {
    pub pubkey: String,
    pub guardians: i64,
    pub queens: i64,
    pub eggs: i64,
}

#[derive(Deserialize, Serialize)]
pub struct Attack {
    pub swarm_pubkey: String,
    pub hive_pubkey: String,
    pub berserkers: i64,
}

#[derive(Deserialize, Serialize)]
pub struct HatchRequest {
    pub pubkey: String,
    pub eggs: i64,
}

pub enum SearchError {
    InvalidPubkey,
    NotFound,
    DBError(MongoError),
}

pub enum StakeError {
    InvalidPubkey,
    NotEnoughTokens,
    DBError(MongoError),
}

impl From<SearchError> for StakeError {
    fn from(e: SearchError) -> StakeError {
        match e {
            SearchError::NotFound => StakeError::NotEnoughTokens,
            SearchError::InvalidPubkey => StakeError::InvalidPubkey,
            SearchError::DBError(e) => StakeError::DBError(e),
        }
    }
}

impl From<mongodb::error::Error> for StakeError {
    fn from(e: mongodb::error::Error) -> StakeError {
        StakeError::DBError(e)
    }
}

impl From<mongodb::error::Error> for SearchError {
    fn from(e: mongodb::error::Error) -> SearchError {
        SearchError::DBError(e)
    }
}

pub enum AirdropError {
    InvalidPubkey,
    AlreadyExists,
    DBError(MongoError),
}

impl From<mongodb::error::Error> for AirdropError {
    fn from(e: mongodb::error::Error) -> AirdropError {
        AirdropError::DBError(e)
    }
}

pub enum AttackError {
    InvalidPubkey,
    NotFound,
    NotEnoughTokens,
    DBError(MongoError),
}

impl From<SearchError> for AttackError {
    fn from(e: SearchError) -> AttackError {
        match e {
            SearchError::NotFound => AttackError::NotEnoughTokens,
            SearchError::InvalidPubkey => AttackError::InvalidPubkey,
            SearchError::DBError(e) => AttackError::DBError(e),
        }
    }
}

impl From<mongodb::error::Error> for AttackError {
    fn from(e: mongodb::error::Error) -> AttackError {
        AttackError::DBError(e)
    }
}

pub trait KeyCloner {
    fn clone_pubkey(&self) -> String;
}

impl KeyCloner for Swarm {
    fn clone_pubkey(&self) -> String {
        self.pubkey.clone()
    }
}

impl KeyCloner for SacredHive {
    fn clone_pubkey(&self) -> String {
        self.pubkey.clone()
    }
}

impl KeyCloner for Hive {
    fn clone_pubkey(&self) -> String {
        self.pubkey.clone()
    }
}

impl KeyCloner for Attack {
    fn clone_pubkey(&self) -> String {
        self.swarm_pubkey.clone()
    }
}

impl KeyCloner for HatchRequest {
    fn clone_pubkey(&self) -> String {
        self.pubkey.clone()
    }
}

pub trait Helpers {
    fn get_collection() -> &'static str;
    fn is_negative(&self) -> bool;
    fn as_swarm(&self) -> Swarm;
    fn add(&mut self, addend: &Self);
    fn negative(&self) -> Self;
}

impl Helpers for Swarm {
    fn is_negative(&self) -> bool {
        self.eggs.is_negative()
            || self.sacred_queens.is_negative()
            || self.queens.is_negative()
            || self.guardians.is_negative()
            || self.berserkers.is_negative()
    }
    fn as_swarm(&self) -> Swarm {
        self.clone()
    }
    fn add(&mut self, addend: &Self) {
        self.sacred_queens += addend.sacred_queens;
        self.queens += addend.queens;
        self.guardians += addend.guardians;
        self.berserkers += addend.berserkers;
        self.eggs += addend.eggs;
    }
    fn negative(&self) -> Self {
        Swarm {
            pubkey: self.pubkey.clone(),
            sacred_queens: -self.sacred_queens,
            eggs: -self.eggs,
            queens: -self.queens,
            guardians: -self.guardians,
            berserkers: -self.berserkers,
        }
    }
    fn get_collection() -> &'static str {
        SWARMS_COLL_NAME
    }
}
impl Helpers for SacredHive {
    fn is_negative(&self) -> bool {
        self.eggs.is_negative() || self.sacred_queens.is_negative()
    }
    fn as_swarm(&self) -> Swarm {
        Swarm {
            pubkey: self.pubkey.clone(),
            sacred_queens: self.sacred_queens,
            eggs: self.eggs,
            queens: 0,
            berserkers: 0,
            guardians: 0,
        }
    }
    fn add(&mut self, addend: &Self) {
        self.sacred_queens += addend.sacred_queens;
        self.eggs += addend.eggs;
    }
    fn negative(&self) -> Self {
        SacredHive {
            pubkey: self.pubkey.clone(),
            sacred_queens: -self.sacred_queens,
            eggs: -self.eggs,
        }
    }
    fn get_collection() -> &'static str {
        SACRED_HIVE_COLL_NAME
    }
}
impl Helpers for Hive {
    fn is_negative(&self) -> bool {
        self.eggs.is_negative() || self.queens.is_negative() || self.guardians.is_negative()
    }
    fn as_swarm(&self) -> Swarm {
        Swarm {
            pubkey: self.pubkey.clone(),
            queens: self.queens,
            guardians: self.guardians,
            eggs: self.eggs,
            sacred_queens: 0,
            berserkers: 0,
        }
    }
    fn add(&mut self, addend: &Self) {
        self.queens += addend.queens;
        self.guardians += addend.guardians;
        self.eggs += addend.eggs;
    }
    fn negative(&self) -> Self {
        Hive {
            pubkey: self.pubkey.clone(),
            eggs: -self.eggs,
            queens: -self.queens,
            guardians: -self.guardians,
        }
    }
    fn get_collection() -> &'static str {
        HIVE_COLL_NAME
    }
}

pub trait Contract:
    Helpers + KeyCloner + DeserializeOwned + Unpin + Send + Sync + Serialize
{
}
impl<T: Helpers + KeyCloner + DeserializeOwned + Unpin + Send + Sync + Serialize> Contract for T {}

pub async fn db_search_hive_top(db: Database) -> Result<Vec<Hive>, SearchError> {
    let coll = db.collection::<Hive>("hives");
    let find_options = FindOptions::builder()
        .limit(10)
        .sort(doc! { "eggs": -1 })
        .build();
    let mut cursor = coll.find(None, find_options).await?;
    let mut hives: Vec<Hive> = Vec::new();
    while let Some(hive) = cursor.try_next().await? {
        hives.push(hive);
    }
    Ok(hives)
}

pub async fn db_search_hive_neigh(neigh: i64, db: Database) -> Result<Vec<Hive>, SearchError> {
    let coll = db.collection::<Hive>("hives");

    let filter = doc! { "eggs": { "$gt": neigh } };
    let find_options = FindOptions::builder()
        .limit(5)
        .sort(doc! { "eggs": 1 })
        .build();
    let mut cursor = coll.find(filter, find_options.clone()).await?;
    let mut hives: Vec<Hive> = Vec::new();
    while let Some(hive) = cursor.try_next().await? {
        hives.push(hive);
    }

    let filter = doc! { "eggs": { "$lte": neigh, "$gt": 0 } };
    let find_options = FindOptions::builder()
        .limit(5)
        .sort(doc! { "eggs": -1 })
        .build();
    let mut cursor = coll.find(filter, find_options).await?;
    while let Some(hive) = cursor.try_next().await? {
        hives.push(hive);
    }

    hives.sort_by(|a, b| b.eggs.cmp(&a.eggs));
    Ok(hives)
}

pub async fn db_search<T: Contract>(pubkey: String, db: Database) -> Result<T, SearchError> {
    if !pubkey_is_valid(&pubkey) {
        return Err(SearchError::InvalidPubkey);
    }
    let collection: Collection<T> = db.collection(T::get_collection());
    match collection.find_one(doc! { "pubkey": &pubkey }, None).await {
        Ok(Some(t)) => Ok(t),
        Ok(None) => Err(SearchError::NotFound),
        Err(err) => Err(SearchError::DBError(err)),
    }
}

async fn db_search_with_session<T: Contract>(
    pubkey: String,
    db: Database,
    session: &mut ClientSession,
) -> Result<T, SearchError> {
    if !pubkey_is_valid(&pubkey) {
        return Err(SearchError::InvalidPubkey);
    }
    let collection: Collection<T> = db.collection(T::get_collection());
    match collection
        .find_one_with_session(doc! { "pubkey": &pubkey }, None, session)
        .await
    {
        Ok(Some(t)) => Ok(t),
        Ok(None) => Err(SearchError::NotFound),
        Err(err) => Err(SearchError::DBError(err)),
    }
}

pub async fn airdrop(pubkey: String, db: Database) -> Result<(), AirdropError> {
    if !pubkey_is_valid(&pubkey) {
        return Err(AirdropError::InvalidPubkey);
    }
    let collection: Collection<Swarm> = db.collection(SWARMS_COLL_NAME);
    match collection.find_one(doc! { "pubkey": &pubkey }, None).await {
        Ok(Some(_)) => Err(AirdropError::AlreadyExists),
        Ok(None) => {
            db.collection(SACRED_HIVE_COLL_NAME)
                .insert_one(
                    SacredHive {
                        pubkey: pubkey.clone(),
                        sacred_queens: 0,
                        eggs: 0,
                    },
                    None,
                )
                .await?;
            db.collection(HIVE_COLL_NAME)
                .insert_one(
                    Hive {
                        pubkey: pubkey.clone(),
                        queens: 0,
                        guardians: 0,
                        eggs: 0,
                    },
                    None,
                )
                .await?;
            collection
                .insert_one(
                    Swarm {
                        berserkers: 0,
                        pubkey,
                        sacred_queens: 10,
                        queens: 0,
                        guardians: 0,
                        eggs: 0,
                    },
                    None,
                )
                .await?;
            Ok(())
        }
        Err(e) => Err(AirdropError::DBError(e)),
    }
}

pub async fn stake<T: Contract>(request: T, mongo_client: Arc<Client>) -> Result<(), StakeError> {
    let mut session = mongo_client.start_session(None).await?;
    session.start_transaction(None).await?;
    let db = mongo_client
        .default_database()
        .expect("default db not specified");
    let mut swarm =
        db_search_with_session::<Swarm>(request.clone_pubkey(), db.clone(), &mut session).await?;
    let mut staked_tokens =
        db_search_with_session::<T>(request.clone_pubkey(), db.clone(), &mut session).await?;
    swarm.add(&request.as_swarm().negative());
    staked_tokens.add(&request);
    if swarm.is_negative() || staked_tokens.is_negative() {
        return Err(StakeError::NotEnoughTokens);
    };
    db.collection::<T>(T::get_collection())
        .replace_one_with_session(
            doc! { "pubkey": request.clone_pubkey() },
            &staked_tokens,
            None,
            &mut session,
        )
        .await?;
    db.collection(Swarm::get_collection())
        .replace_one_with_session(
            doc! { "pubkey": request.clone_pubkey() },
            swarm,
            None,
            &mut session,
        )
        .await?;
    session.commit_transaction().await?;
    Ok(())
}

pub async fn unstake<T: Contract>(request: T, mongo_client: Arc<Client>) -> Result<(), StakeError> {
    stake::<T>(request.negative(), mongo_client).await
}

pub async fn trigger(pubkey: String, mongo_client: Arc<Client>) -> Result<(), SearchError> {
    let mut session = mongo_client.start_session(None).await?;
    session.start_transaction(None).await?;
    let db = mongo_client
        .default_database()
        .expect("default db not specified");
    let mut sacred_hive =
        db_search_with_session::<SacredHive>(pubkey, db.clone(), &mut session)
            .await?;
    sacred_hive.eggs += sacred_hive.sacred_queens * 100;
    db.collection(SacredHive::get_collection())
        .replace_one_with_session(
            doc! { "pubkey": sacred_hive.clone_pubkey() },
            sacred_hive,
            None,
            &mut session,
        )
        .await?;
    session.commit_transaction().await?;
    Ok(())
}

pub async fn process_hatch_request(
    request: HatchRequest,
    mongo_client: Arc<Client>,
) -> Result<(), StakeError> {
    let mut session = mongo_client.start_session(None).await?;
    session.start_transaction(None).await?;
    let db = mongo_client
        .default_database()
        .expect("default db not specified");
    let mut swarm =
        db_search_with_session::<Swarm>(request.pubkey.clone(), db.clone(), &mut session).await?;
    if swarm.eggs < request.eggs {
        return Err(StakeError::NotEnoughTokens);
    }
    swarm.eggs -= request.eggs;
    let mut eggs = request.eggs;
    while eggs > 0 {
        let drop = rand::random::<u64>() % 100;
        if drop == 0 {
            swarm.queens += 1;
        } else if drop < 10 {
            swarm.guardians += 1;
        } else {
            swarm.berserkers += 1;
        }
        eggs -= 1;
    }
    db.collection(Swarm::get_collection())
        .replace_one_with_session(
            doc! { "pubkey": swarm.clone_pubkey() },
            swarm,
            None,
            &mut session,
        )
        .await?;
    session.commit_transaction().await?;
    Ok(())
}

pub async fn attack(request: Attack, mongo_client: Arc<Client>) -> Result<(), AttackError> {
    let mut session = mongo_client.start_session(None).await?;
    session.start_transaction(None).await?;
    let db = mongo_client
        .default_database()
        .expect("default db not specified");
    let mut swarm =
        db_search_with_session::<Swarm>(request.swarm_pubkey.clone(), db.clone(), &mut session)
            .await?;
    let mut hive =
        db_search_with_session::<Hive>(request.hive_pubkey.clone(), db.clone(), &mut session)
            .await?;
    if swarm.berserkers < request.berserkers {
        return Err(AttackError::NotEnoughTokens);
    }
    swarm.berserkers -= request.berserkers;
    if hive.eggs == 0 {
        return Err(AttackError::NotFound);
    }
    let attack_power = request.berserkers;
    let random_queen_defense: i64 = i64::from(rand::random::<u8>()) % 10;
    let random_guardian_defense: i64 = i64::from(rand::random::<u8>()) % 2;
    let defense_power =
        (hive.queens * random_queen_defense) + (hive.guardians * (9 + random_guardian_defense));
    if attack_power > defense_power {
        swarm.eggs += hive.eggs;
        hive.eggs = 0;
        hive.queens = 0;
        hive.guardians = 0;
    }
    db.collection(Hive::get_collection())
        .replace_one_with_session(
            doc! { "pubkey": hive.clone_pubkey() },
            hive,
            None,
            &mut session,
        )
        .await?;
    db.collection(Swarm::get_collection())
        .replace_one_with_session(
            doc! { "pubkey": swarm.clone_pubkey() },
            swarm,
            None,
            &mut session,
        )
        .await?;
    session.commit_transaction().await?;
    Ok(())
}

pub async fn create_db_indexes(db: &Database) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "pubkey": 1 })
        .options(options)
        .build();

    db.collection::<Swarm>(Swarm::get_collection())
        .create_index(model.clone(), None)
        .await
        .expect("creating an index should succeed");

    db.collection::<SacredHive>(SacredHive::get_collection())
        .create_index(model.clone(), None)
        .await
        .expect("creating an index should succeed");

    db.collection::<Hive>(Hive::get_collection())
        .create_index(model, None)
        .await
        .expect("creating an index should succeed");
}

pub async fn init_mockup_db(db: &Database) {
    match db
        .collection::<Swarm>(Swarm::get_collection())
        .find_one(None, None)
        .await
    {
        Ok(None) => add_random_data(&db).await,
        _ => {}
    }
}

async fn add_random_data(db: &Database) {
    let n = 999;
    let mut swarms: Vec<Swarm> = Vec::new();
    let mut hives: Vec<Hive> = Vec::new();
    let mut sacred_hives: Vec<SacredHive> = Vec::new();
    for _ in 0..n {
        let pubkey = get_pubkey(&generate_keypair());
        let m = rand::random::<u64>() % 10;
        swarms.push(Swarm {
            pubkey: pubkey.clone(),
            sacred_queens: (rand::random::<u64>() % 10 * m * (rand::random::<u64>() % 2)) as i64,
            queens: (rand::random::<u64>() % 100 * m * (rand::random::<u64>() % 2)) as i64,
            guardians: (rand::random::<u64>() % 1000 * m * (rand::random::<u64>() % 2)) as i64,
            eggs: (rand::random::<u64>() % 10000 * (rand::random::<u64>() % 2)) as i64,
            berserkers: (rand::random::<u64>() % 10000 * m * (rand::random::<u64>() % 2)) as i64,
        });
        let m = rand::random::<u64>() % 10;
        hives.push(Hive {
            pubkey: pubkey.clone(),
            guardians: (rand::random::<u64>() % 1000 * m + 1) as i64,
            queens: (rand::random::<u64>() % 100 * m + 10) as i64,
            eggs: (rand::random::<u64>() % 1000 * m) as i64,
        });
        let m = rand::random::<u64>() % 10;
        sacred_hives.push(SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: (rand::random::<u64>() % 10 * m) as i64,
            eggs: (rand::random::<u64>() % 100) as i64,
        });
    }
    db.collection(Swarm::get_collection())
        .insert_many(swarms.clone(), None)
        .await
        .unwrap();
    db.collection(Hive::get_collection())
        .insert_many(hives.clone(), None)
        .await
        .unwrap();
    db.collection(SacredHive::get_collection())
        .insert_many(sacred_hives.clone(), None)
        .await
        .unwrap();
}

fn pubkey_is_valid(pubkey: &str) -> bool {
    bs58::decode(pubkey)
        .into_vec()
        .ok()
        .and_then(|key| key.try_into().ok())
        .and_then(|decoded_pubkey: [u8; PUBLIC_KEY_LENGTH]| {
            PublicKey::from_bytes(&decoded_pubkey).ok()
        })
        .is_some()
}

pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng {};
    Keypair::generate(&mut csprng)
}

pub fn get_pubkey(keypair: &Keypair) -> String {
    bs58::encode(keypair.public.to_bytes()).into_string()
}
