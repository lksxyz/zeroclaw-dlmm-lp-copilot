//! Solana JSON-RPC + Jupiter price via waki (blocking wasi:http).
//!
//! Wasm-only: compiled only for the plugin component. On the host these are
//! error stubs so `cargo test` works without a network (same pattern as the
//! spike-http probe).

use solana_pubkey::Pubkey;

pub struct RpcClient {
    url: String,
}

impl RpcClient {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    /// getAccountInfo with base64 encoding — returns the raw account data.
    pub fn get_account_info(&self, key: &Pubkey) -> Result<Option<Vec<u8>>, String> {
        rpc_get_account_info(&self.url, key)
    }

    /// Active bin id of an LbPair account.
    pub fn get_active_bin(&self, pool: &Pubkey) -> Result<i32, String> {
        let data = self
            .get_account_info(pool)?
            .ok_or_else(|| format!("pool account not found: {pool}"))?;
        crate::decoder::decode_lb_pair(&data)
            .map(|lb| lb.active_id)
            .map_err(|e| format!("bad lb_pair data: {e}"))
    }

    /// Stored durable-nonce hash (base58) — the tx's recentBlockhash.
    pub fn get_nonce_hash(&self, nonce: &Pubkey) -> Result<String, String> {
        let data = self
            .get_account_info(nonce)?
            .ok_or_else(|| format!("nonce account not found: {nonce}"))?;
        crate::nonce::nonce_hash_base58(&data).map_err(|e| format!("bad nonce data: {e}"))
    }

    /// Stored durable-nonce hash as raw bytes — the tx's recentBlockhash.
    pub fn get_nonce_hash_bytes(&self, nonce: &Pubkey) -> Result<[u8; 32], String> {
        let data = self
            .get_account_info(nonce)?
            .ok_or_else(|| format!("nonce account not found: {nonce}"))?;
        crate::nonce::decode_nonce_hash(&data).map_err(|e| format!("bad nonce data: {e}"))
    }

    /// Latest blockhash as raw 32 bytes — used as `recentBlockhash` when no
    /// durable nonce is configured. Blockhashes expire after ~90 s, so the
    /// user must sign promptly; the doc-comment on `build_action` describes
    /// why a durable nonce is still the recommended path.
    pub fn get_latest_blockhash_bytes(&self) -> Result<[u8; 32], String> {
        rpc_get_latest_blockhash(&self.url)
    }

    /// Token-mint decimals (SPL layout: u8 at offset 44). Needed to convert
    /// raw claimable amounts to human units.
    pub fn get_mint_decimals(&self, mint: &Pubkey) -> Result<u8, String> {
        let data = self
            .get_account_info(mint)?
            .ok_or_else(|| format!("mint not found: {mint}"))?;
        data.get(44)
            .copied()
            .ok_or_else(|| format!("bad mint data: {mint}"))
    }

    /// All accounts of `program` with `data_size` bytes whose bytes at
    /// `memcmp_offset` equal `memcmp_bytes` (base58-encoded memcmp filter).
    /// Returns (pubkey, data) pairs. Used by the reader's position-discovery
    /// mode (PositionV2: owner at offset 40).
    pub fn get_program_accounts(
        &self,
        program: &Pubkey,
        data_size: u64,
        memcmp_offset: usize,
        memcmp_bytes: &[u8],
    ) -> Result<Vec<(Pubkey, Vec<u8>)>, String> {
        rpc_get_program_accounts(&self.url, program, data_size, memcmp_offset, memcmp_bytes)
    }
}

#[cfg(target_family = "wasm")]
fn rpc_get_account_info(url: &str, key: &Pubkey) -> Result<Option<Vec<u8>>, String> {
    use base64::Engine;
    use serde_json::json;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [key.to_string(), { "encoding": "base64", "commitment": "confirmed" }]
    })
    .to_string();

    let resp = waki::Client::new()
        .post(url)
        .header("content-type", "application/json")
        .body(body.as_bytes().to_vec())
        .connect_timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("rpc error: {e}"))?;

    if resp.status_code() != 200 {
        return Err(format!("rpc status {}", resp.status_code()));
    }
    let body = resp.body().map_err(|e| format!("rpc body error: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("rpc json error: {e}"))?;
    let Some(data) = v["result"]["value"].as_object() else {
        return Ok(None);
    };
    let Some(enc) = data.get("data") else {
        return Ok(None);
    };
    let (b64, _) = enc
        .as_array()
        .and_then(|a| a.first())
        .and_then(|e| e.as_str())
        .zip(enc.as_array().and_then(|a| a.get(1)))
        .ok_or_else(|| "unexpected data encoding".to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 decode: {e}"))?;
    Ok(Some(bytes))
}

/// Jupiter Price API (v2) — replaces the deprecated Pyth feeds.
pub fn get_jupiter_price(token_mint: &Pubkey) -> Result<f64, String> {
    jupiter_price(token_mint)
}

#[cfg(target_family = "wasm")]
fn rpc_get_program_accounts(
    url: &str,
    program: &Pubkey,
    data_size: u64,
    memcmp_offset: usize,
    memcmp_bytes: &[u8],
) -> Result<Vec<(Pubkey, Vec<u8>)>, String> {
    use base64::Engine;
    use serde_json::json;

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [program.to_string(), {
            "encoding": "base64",
            "commitment": "confirmed",
            "filters": [
                { "dataSize": data_size },
                { "memcmp": { "offset": memcmp_offset, "bytes": bs58::encode(memcmp_bytes).into_string() } }
            ]
        }]
    })
    .to_string();

    let resp = waki::Client::new()
        .post(url)
        .header("content-type", "application/json")
        .body(body.as_bytes().to_vec())
        .connect_timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("rpc error: {e}"))?;
    if resp.status_code() != 200 {
        return Err(format!("rpc status {}", resp.status_code()));
    }
    let resp_body = resp.body().map_err(|e| format!("rpc body error: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_slice(&resp_body).map_err(|e| format!("rpc json error: {e}"))?;

    let Some(accounts) = v["result"].as_array() else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(accounts.len());
    for acc in accounts {
        let Some(key) = acc["pubkey"].as_str() else { continue };
        let Some(data) = acc["account"]["data"].as_array().and_then(|a| a.first()).and_then(|e| e.as_str()) else {
            continue;
        };
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) else {
            continue;
        };
        let Ok(pk) = key.parse() else { continue };
        out.push((pk, bytes));
    }
    Ok(out)
}

#[cfg(not(target_family = "wasm"))]
fn rpc_get_program_accounts(
    _url: &str,
    _program: &Pubkey,
    _data_size: u64,
    _memcmp_offset: usize,
    _memcmp_bytes: &[u8],
) -> Result<Vec<(Pubkey, Vec<u8>)>, String> {
    Err("rpc: host stub — wasm-only".to_string())
}

/// Parse a `getLatestBlockhash` JSON response into a 32-byte blockhash.
///
/// Solana RPC returns the blockhash as a base58 string (e.g.
/// `"9CmjKoWGqndHBRmRFL6jcc6cZs9cQttNkpTY4Y8d5vJ3"`) — **not** base64.
/// Decoding a base58 alphabet string as base64 yields 33 bytes and trips the
/// length check; that was the source of the
/// `"blockhash not 32 bytes: 33"` failure on the no-nonce demo path.
pub fn decode_blockhash_from_response(v: &serde_json::Value) -> Result<[u8; 32], String> {
    let b58_str = v["result"]["value"]["blockhash"]
        .as_str()
        .ok_or_else(|| "no blockhash in response".to_string())?;
    let bytes = bs58::decode(b58_str)
        .into_vec()
        .map_err(|e| format!("blockhash base58: {e}"))?;
    if bytes.len() != 32 {
        return Err(format!("blockhash not 32 bytes: {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

#[cfg(target_family = "wasm")]
fn rpc_get_latest_blockhash(url: &str) -> Result<[u8; 32], String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getLatestBlockhash",
        "params": [{ "commitment": "confirmed" }]
    })
    .to_string();
    let resp = waki::Client::new()
        .post(url)
        .header("content-type", "application/json")
        .body(body.as_bytes().to_vec())
        .connect_timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("rpc error: {e}"))?;
    if resp.status_code() != 200 {
        return Err(format!("rpc status {}", resp.status_code()));
    }
    let body = resp.body().map_err(|e| format!("rpc body error: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("rpc json error: {e}"))?;
    decode_blockhash_from_response(&v)
}

#[cfg(not(target_family = "wasm"))]
fn rpc_get_latest_blockhash(_url: &str) -> Result<[u8; 32], String> {
    Err("rpc: host stub — wasm-only".to_string())
}

#[cfg(target_family = "wasm")]
fn jupiter_price(token_mint: &Pubkey) -> Result<f64, String> {
    let url = format!("https://api.jup.ag/price/v2?ids={token_mint}");
    let resp = waki::Client::new()
        .get(&url)
        .connect_timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| format!("price error: {e}"))?;
    let body = resp.body().map_err(|e| format!("price body error: {e}"))?;
    let v: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| format!("price json error: {e}"))?;
    let price = v["data"][token_mint.to_string()]["price"]
        .as_str()
        .ok_or_else(|| format!("no price for {token_mint}"))?
        .parse::<f64>()
        .map_err(|e| format!("bad price: {e}"))?;
    Ok(price)
}

// ------------------------- host stubs --------------------------------------

#[cfg(not(target_family = "wasm"))]
fn rpc_get_account_info(_url: &str, _key: &Pubkey) -> Result<Option<Vec<u8>>, String> {
    Err("rpc: host stub — wasm-only".to_string())
}

#[cfg(not(target_family = "wasm"))]
fn jupiter_price(_token_mint: &Pubkey) -> Result<f64, String> {
    Err("price: host stub — wasm-only".to_string())
}
