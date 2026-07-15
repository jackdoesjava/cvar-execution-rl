use std::collections::BTreeMap;
use dbn::{decode::dbn::Decoder, record::MboMsg};
use dbn::decode::DecodeRecord; // Bring the standard decode trait into scope
use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

#[pyclass]
pub struct OrderBook {
    bids: BTreeMap<i64, BTreeMap<u64, u32>>, 
    asks: BTreeMap<i64, BTreeMap<u64, u32>>,
}

#[pymethods]
impl OrderBook {
    #[new]
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Iterates through the Databento DBN file and reconstructs the L3 book state
    pub fn ingest_dbn(&mut self, file_path: &str) -> PyResult<()> {
        let mut decoder = Decoder::from_zstd_file(file_path)
            .map_err(|e| PyRuntimeError::new_err(format!("File error: {}", e)))?;
        
        // Decode record-by-record instead of relying on a stream iterator
        while let Some(msg) = decoder.decode_record::<MboMsg>()
            .map_err(|e| PyRuntimeError::new_err(format!("Decode error: {}", e)))? 
        {
            let price = msg.price;
            let order_id = msg.order_id;
            let size = msg.size;
            
            // Databento uses 'A' (Ask) and 'B' (Bid)
            let is_ask = (msg.side as u8 as char) == 'A'; 
            let target_side = if is_ask { &mut self.asks } else { &mut self.bids };

            match msg.action as u8 as char {
                'A' => {
                    target_side.entry(price).or_insert_with(BTreeMap::new).insert(order_id, size);
                }
                'C' | 'E' => {
                    if let Some(level) = target_side.get_mut(&price) {
                        level.remove(&order_id);
                        if level.is_empty() { target_side.remove(&price); }
                    }
                }
                'M' => {
                    if let Some(level) = target_side.get_mut(&price) {
                        if let Some(existing_size) = level.get_mut(&order_id) {
                            *existing_size = size;
                        }
                    }
                }
                _ => {} 
            }
        }
        Ok(())
    }

    /// Fast getter for the RL state tensor: Top N bid prices and their total volume
    pub fn get_top_bids(&self, n: usize) -> Vec<(i64, u32)> {
        self.bids.iter().rev().take(n).map(|(&price, orders)| {
            let total_vol: u32 = orders.values().sum();
            (price, total_vol)
        }).collect()
    }

    /// Fast getter for the RL state tensor: Top N ask prices and their total volume
    pub fn get_top_asks(&self, n: usize) -> Vec<(i64, u32)> {
        self.asks.iter().take(n).map(|(&price, orders)| {
            let total_vol: u32 = orders.values().sum();
            (price, total_vol)
        }).collect()
    }
}