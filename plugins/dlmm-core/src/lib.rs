//! dlmm-core — shared pure core for the DLMM plugin suite.
//!
//! Decodes Meteora DLMM on-chain accounts (PositionV2 / LbPair / BinArray —
//! bytemuck C-repr layouts, NOT the borsh layout of older docs), builds
//! claim / rebalance instructions + durable-nonce transactions, derives
//! program PDAs, and applies the mechanical validation rules.
//!
//! No wasm imports in this crate except the optional `rpc` module (waki,
//! wasm-only, host stub otherwise) — everything else is host-testable with
//! `cargo test`. The thin shims `dlmm-reader` and `dlmm-builder` depend on
//! this and add only the ZeroClaw WIT glue.

pub mod decoder;
pub mod nonce;
pub mod pda;
pub mod rpc;
pub mod tx;
pub mod validate;
