//! WebAssembly bindings for the Chia DataLayer driver.
//!
//! Mirrors the offline subset of the NAPI interface. Networking
//! (Peer/Tls) is intentionally absent — WASM has no native sockets.

use wasm_bindgen::prelude::*;

mod types;

/// Initialise the module. Call once at startup. Installs a panic hook
/// (when the `console-panic-hook` feature is on) so Rust panics surface
/// in the JS console instead of an opaque `unreachable`.
#[wasm_bindgen]
pub fn init() {
    #[cfg(feature = "console-panic-hook")]
    console_error_panic_hook::set_once();
}

use datalayer_driver::{
    address_to_puzzle_hash as core_address_to_puzzle_hash,
    admin_delegated_puzzle_from_key as core_admin_dp, get_coin_id as core_get_coin_id,
    get_mainnet_genesis_challenge as core_mainnet_gc,
    get_testnet11_genesis_challenge as core_testnet_gc,
    master_public_key_to_first_puzzle_hash as core_mpk_to_first_ph,
    master_public_key_to_wallet_synthetic_key as core_mpk_to_synth,
    master_secret_key_to_wallet_synthetic_secret_key as core_msk_to_synth,
    morph_launcher_id_wrapper as core_morph_launcher_id,
    puzzle_hash_to_address as core_ph_to_address, secret_key_to_public_key as core_sk_to_pk,
    synthetic_key_to_puzzle_hash as core_synth_to_ph,
    writer_delegated_puzzle_from_key as core_writer_dp,
};

use crate::types::{
    bytes32, from_js, public_key, secret_key, to_js, Coin, DelegatedPuzzle, EveProof, LineageProof,
    Proof,
};

// ── Task 6: DIGStore spend builders ──────────────────────────────────────────
use crate::types::{
    coin_spends_to_js, coins_from_js, delegated_puzzles_from_js, DataStore, SuccessResponse,
};
use datalayer_driver::{
    melt_store as core_melt_store, mint_store as core_mint_store,
    oracle_spend as core_oracle_spend, update_store_metadata as core_update_meta,
    update_store_ownership as core_update_owner, DataStoreInnerSpend,
};

// ── Task 7: Signing / serialization / selection / server-coin ────────────────
use crate::types::{coin_spends_from_js, signature, NewServerCoin, Output, ServerCoin};
use datalayer_driver::{
    add_fee as core_add_fee, create_server_coin as core_create_server_coin,
    get_cost as core_get_cost, hex_spend_bundle_to_coin_spends as core_hex_to_css,
    select_coins as core_select_coins, send_xch as core_send_xch,
    sign_coin_spends as core_sign_css, sign_message as core_sign_msg,
    spend_bundle_to_hex as core_sb_to_hex, verify_signed_message as core_verify_msg,
    Bytes as RustBytes, Output as RustOutput, SpendBundle as RustSpendBundle,
};

#[wasm_bindgen(js_name = "masterPublicKeyToWalletSyntheticKey")]
pub fn master_public_key_to_wallet_synthetic_key(
    public_key_bytes: &[u8],
) -> Result<Vec<u8>, JsValue> {
    Ok(core_mpk_to_synth(&public_key(public_key_bytes)?)
        .to_bytes()
        .to_vec())
}

#[wasm_bindgen(js_name = "masterPublicKeyToFirstPuzzleHash")]
pub fn master_public_key_to_first_puzzle_hash(public_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    Ok(core_mpk_to_first_ph(&public_key(public_key_bytes)?).to_vec())
}

#[wasm_bindgen(js_name = "masterSecretKeyToWalletSyntheticSecretKey")]
pub fn master_secret_key_to_wallet_synthetic_secret_key(
    secret_key_bytes: &[u8],
) -> Result<Vec<u8>, JsValue> {
    Ok(core_msk_to_synth(&secret_key(secret_key_bytes)?)
        .to_bytes()
        .to_vec())
}

#[wasm_bindgen(js_name = "secretKeyToPublicKey")]
pub fn secret_key_to_public_key(secret_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    Ok(core_sk_to_pk(&secret_key(secret_key_bytes)?)
        .to_bytes()
        .to_vec())
}

#[wasm_bindgen(js_name = "syntheticKeyToPuzzleHash")]
pub fn synthetic_key_to_puzzle_hash(synthetic_key_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    Ok(core_synth_to_ph(&public_key(synthetic_key_bytes)?).to_vec())
}

#[wasm_bindgen(js_name = "puzzleHashToAddress")]
pub fn puzzle_hash_to_address(puzzle_hash: &[u8], prefix: String) -> Result<String, JsValue> {
    core_ph_to_address(bytes32(puzzle_hash)?, &prefix)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "addressToPuzzleHash")]
pub fn address_to_puzzle_hash(address: String) -> Result<Vec<u8>, JsValue> {
    core_address_to_puzzle_hash(&address)
        .map(|b| b.to_vec())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "adminDelegatedPuzzleFromKey")]
pub fn admin_delegated_puzzle_from_key(synthetic_key: &[u8]) -> Result<JsValue, JsValue> {
    to_js(&DelegatedPuzzle::from_native(&core_admin_dp(&public_key(
        synthetic_key,
    )?))?)
}

#[wasm_bindgen(js_name = "writerDelegatedPuzzleFromKey")]
pub fn writer_delegated_puzzle_from_key(synthetic_key: &[u8]) -> Result<JsValue, JsValue> {
    to_js(&DelegatedPuzzle::from_native(&core_writer_dp(
        &public_key(synthetic_key)?,
    ))?)
}

#[wasm_bindgen(js_name = "newLineageProof")]
pub fn new_lineage_proof(lineage_proof: JsValue) -> Result<JsValue, JsValue> {
    let lp: LineageProof = from_js(lineage_proof)?;
    to_js(&Proof {
        lineage_proof: Some(lp),
        eve_proof: None,
    })
}

#[wasm_bindgen(js_name = "newEveProof")]
pub fn new_eve_proof(eve_proof: JsValue) -> Result<JsValue, JsValue> {
    let ep: EveProof = from_js(eve_proof)?;
    to_js(&Proof {
        lineage_proof: None,
        eve_proof: Some(ep),
    })
}

#[wasm_bindgen(js_name = "getCoinId")]
pub fn get_coin_id(coin: JsValue) -> Result<Vec<u8>, JsValue> {
    let c: Coin = from_js(coin)?;
    Ok(core_get_coin_id(&c.to_native()?).to_vec())
}

#[wasm_bindgen(js_name = "morphLauncherId")]
pub fn morph_launcher_id(launcher_id: &[u8], offset: u64) -> Result<Vec<u8>, JsValue> {
    Ok(core_morph_launcher_id(bytes32(launcher_id)?, offset).to_vec())
}

#[wasm_bindgen(js_name = "getMainnetGenesisChallenge")]
pub fn get_mainnet_genesis_challenge() -> Vec<u8> {
    core_mainnet_gc().to_vec()
}

#[wasm_bindgen(js_name = "getTestnet11GenesisChallenge")]
pub fn get_testnet11_genesis_challenge() -> Vec<u8> {
    core_testnet_gc().to_vec()
}

// ── Task 6: DIGStore spend builders ──────────────────────────────────────────

#[wasm_bindgen(js_name = "mintStore")]
#[allow(clippy::too_many_arguments)]
pub fn mint_store(
    minter_synthetic_key: &[u8],
    selected_coins: JsValue,
    root_hash: &[u8],
    label: Option<String>,
    description: Option<String>,
    bytes: Option<u64>,
    size_proof: Option<Vec<u8>>,
    owner_puzzle_hash: &[u8],
    delegated_puzzles: JsValue,
    fee: u64,
) -> Result<JsValue, JsValue> {
    let size_proof = match size_proof {
        Some(sp) => Some(bytes32(&sp)?.to_string()),
        None => None,
    };
    let resp = core_mint_store(
        public_key(minter_synthetic_key)?,
        coins_from_js(selected_coins)?,
        bytes32(root_hash)?,
        label,
        description,
        bytes,
        size_proof,
        bytes32(owner_puzzle_hash)?,
        delegated_puzzles_from_js(delegated_puzzles)?,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

#[wasm_bindgen(js_name = "oracleSpend")]
pub fn oracle_spend(
    spender_synthetic_key: &[u8],
    selected_coins: JsValue,
    store: JsValue,
    fee: u64,
) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let resp = core_oracle_spend(
        public_key(spender_synthetic_key)?,
        coins_from_js(selected_coins)?,
        store.to_native()?,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

#[wasm_bindgen(js_name = "meltStore")]
pub fn melt_store(store: JsValue, owner_public_key: &[u8]) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let css = core_melt_store(store.to_native()?, public_key(owner_public_key)?)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}

#[wasm_bindgen(js_name = "updateStoreMetadata")]
#[allow(clippy::too_many_arguments)]
pub fn update_store_metadata(
    store: JsValue,
    new_root_hash: &[u8],
    new_label: Option<String>,
    new_description: Option<String>,
    new_bytes: Option<u64>,
    new_size_proof: Option<Vec<u8>>,
    owner_public_key: Option<Vec<u8>>,
    admin_public_key: Option<Vec<u8>>,
    writer_public_key: Option<Vec<u8>>,
) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let inner =
        match (&owner_public_key, &admin_public_key, &writer_public_key) {
            (Some(pk), None, None) => DataStoreInnerSpend::Owner(public_key(pk)?),
            (None, Some(pk), None) => DataStoreInnerSpend::Admin(public_key(pk)?),
            (None, None, Some(pk)) => DataStoreInnerSpend::Writer(public_key(pk)?),
            _ => return Err(JsValue::from_str(
                "Exactly one of ownerPublicKey, adminPublicKey, writerPublicKey must be provided",
            )),
        };
    let new_size_proof = match new_size_proof {
        Some(sp) => Some(bytes32(&sp)?.to_string()),
        None => None,
    };
    let resp = core_update_meta(
        store.to_native()?,
        bytes32(new_root_hash)?,
        new_label,
        new_description,
        new_bytes,
        new_size_proof,
        inner,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

#[wasm_bindgen(js_name = "updateStoreOwnership")]
pub fn update_store_ownership(
    store: JsValue,
    new_owner_puzzle_hash: Option<Vec<u8>>,
    new_delegated_puzzles: JsValue,
    owner_public_key: Option<Vec<u8>>,
    admin_public_key: Option<Vec<u8>>,
) -> Result<JsValue, JsValue> {
    let store: DataStore = from_js(store)?;
    let native_store = store.to_native()?;
    let new_owner_ph = match new_owner_puzzle_hash {
        Some(ph) => bytes32(&ph)?,
        None => native_store.info.owner_puzzle_hash,
    };
    let inner = match (&owner_public_key, &admin_public_key) {
        (Some(pk), None) => DataStoreInnerSpend::Owner(public_key(pk)?),
        (None, Some(pk)) => DataStoreInnerSpend::Admin(public_key(pk)?),
        _ => {
            return Err(JsValue::from_str(
                "Exactly one of ownerPublicKey, adminPublicKey must be provided",
            ))
        }
    };
    let resp = core_update_owner(
        native_store,
        new_owner_ph,
        delegated_puzzles_from_js(new_delegated_puzzles)?,
        inner,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    to_js(&SuccessResponse::from_native(&resp)?)
}

// ── Task 7 functions ─────────────────────────────────────────────────────────

#[wasm_bindgen(js_name = "signCoinSpends")]
pub fn sign_coin_spends(
    coin_spends: JsValue,
    private_keys: JsValue,
    for_testnet: bool,
) -> Result<Vec<u8>, JsValue> {
    let css = coin_spends_from_js(coin_spends)?;
    let keys_raw: Vec<serde_bytes::ByteBuf> = from_js(private_keys)?;
    let keys = keys_raw
        .iter()
        .map(|k| secret_key(k))
        .collect::<Result<Vec<_>, _>>()?;
    let sig =
        core_sign_css(&css, &keys, for_testnet).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(sig.to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "signMessage")]
pub fn sign_message(message: &[u8], private_key: &[u8]) -> Result<Vec<u8>, JsValue> {
    let sig = core_sign_msg(message, &secret_key(private_key)?)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(sig.to_bytes().to_vec())
}

#[wasm_bindgen(js_name = "verifySignedMessage")]
pub fn verify_signed_message(
    sig: &[u8],
    public_key_bytes: &[u8],
    message: &[u8],
) -> Result<bool, JsValue> {
    core_verify_msg(&signature(sig)?, &public_key(public_key_bytes)?, message)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "getCost")]
pub fn get_cost(coin_spends: JsValue) -> Result<u64, JsValue> {
    core_get_cost(&coin_spends_from_js(coin_spends)?).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "selectCoins")]
pub fn select_coins(all_coins: JsValue, total_amount: u64) -> Result<JsValue, JsValue> {
    let selected = core_select_coins(&coins_from_js(all_coins)?, total_amount)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out: Vec<crate::types::Coin> = selected
        .iter()
        .map(crate::types::Coin::from_native)
        .collect();
    to_js(&out)
}

#[wasm_bindgen(js_name = "spendBundleToHex")]
pub fn spend_bundle_to_hex(spend_bundle: JsValue) -> Result<String, JsValue> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SbIn {
        coin_spends: Vec<crate::types::CoinSpend>,
        #[serde(with = "serde_bytes")]
        aggregated_signature: Vec<u8>,
    }
    let sb: SbIn = from_js(spend_bundle)?;
    let css = sb
        .coin_spends
        .iter()
        .map(crate::types::CoinSpend::to_native)
        .collect::<Result<Vec<_>, _>>()?;
    let bundle = RustSpendBundle::new(css, signature(&sb.aggregated_signature)?);
    core_sb_to_hex(&bundle).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen(js_name = "hexSpendBundleToCoinSpends")]
pub fn hex_spend_bundle_to_coin_spends(hex: String) -> Result<JsValue, JsValue> {
    let css = core_hex_to_css(&hex).map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}

#[wasm_bindgen(js_name = "sendXch")]
pub fn send_xch(
    synthetic_key: &[u8],
    selected_coins: JsValue,
    outputs: JsValue,
    fee: u64,
) -> Result<JsValue, JsValue> {
    let outs: Vec<Output> = from_js(outputs)?;
    let native_outs: Vec<RustOutput> = outs
        .iter()
        .map(|o| {
            Ok(RustOutput {
                puzzle_hash: bytes32(&o.puzzle_hash)?,
                amount: o.amount,
                memos: o
                    .memos
                    .iter()
                    .map(|m| RustBytes::from(m.to_vec()))
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, JsValue>>()?;
    let css = core_send_xch(
        &public_key(synthetic_key)?,
        &coins_from_js(selected_coins)?,
        &native_outs,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}

#[wasm_bindgen(js_name = "addFee")]
pub fn add_fee(
    spender_synthetic_key: &[u8],
    selected_coins: JsValue,
    assert_coin_ids: JsValue,
    fee: u64,
) -> Result<JsValue, JsValue> {
    let ids_raw: Vec<serde_bytes::ByteBuf> = from_js(assert_coin_ids)?;
    let ids = ids_raw
        .iter()
        .map(|b| bytes32(b))
        .collect::<Result<Vec<_>, _>>()?;
    let css = core_add_fee(
        &public_key(spender_synthetic_key)?,
        &coins_from_js(selected_coins)?,
        &ids,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    coin_spends_to_js(&css)
}

#[wasm_bindgen(js_name = "createServerCoin")]
pub fn create_server_coin(
    synthetic_key: &[u8],
    selected_coins: JsValue,
    hint: &[u8],
    uris: JsValue,
    amount: u64,
    fee: u64,
) -> Result<JsValue, JsValue> {
    let uris: Vec<String> = from_js(uris)?;
    let new_sc = core_create_server_coin(
        public_key(synthetic_key)?,
        coins_from_js(selected_coins)?,
        bytes32(hint)?,
        uris,
        amount,
        fee,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = NewServerCoin {
        server_coin: ServerCoin::from_native(&new_sc.server_coin),
        coin_spends: new_sc
            .coin_spends
            .iter()
            .map(crate::types::CoinSpend::from_native)
            .collect(),
    };
    to_js(&out)
}
