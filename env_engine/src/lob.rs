use std::collections::BTreeMap;
use std::error::Error;
use dbn::{decode::dbn::Decoder, record::MboMsg};

// BTreeMap keeps our price levels automatically sorted.
// asks: Lowest price first (standard B-Tree behavior)
// bids: Highest price first (we'll handle reading this in reverse later)
pub struct OrderBook {
    // Map: Price -> (Order_ID -> Size)
    pub bids: BTreeMap<i64, BTreeMap<u64, u32>>, 
    pub asks: BTreeMap<i64, BTreeMap<u64, u32>>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// Streams the Databento DBN file and reconstructs the L3 book state
    pub fn ingest_dbn(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        // Native zstd decoding. Do not decompress to CSV first!
        let mut decoder = Decoder::from_zstd_file(file_path)?;
        let mut mbo_stream = decoder.decode_stream::<MboMsg>()?;

        println!("Streaming MBO data... this might take a second.");

        while let Some(msg) = mbo_stream.next() {
            let price = msg.price;
            let order_id = msg.order_id;
            let size = msg.size;
            
            // Databento uses 'A' for Ask, 'B' for Bid
            let is_ask = msg.side == 'A'; 
            let target_side = if is_ask { &mut self.asks } else { &mut self.bids };

            match msg.action as char {
                'A' => {
                    // Add order to the book
                    target_side
                        .entry(price)
                        .or_insert_with(BTreeMap::new)
                        .insert(order_id, size);
                }
                'C' | 'E' => {
                    // Cancel or Execute: Both remove liquidity from the book
                    if let Some(level) = target_side.get_mut(&price) {
                        level.remove(&order_id);
                        
                        // Clean up the price level if it's empty to keep tree traversal fast
                        if level.is_empty() {
                            target_side.remove(&price);
                        }
                    }
                }
                'M' => {
                    // Modify size (usually a partial fill or partial cancel)
                    if let Some(level) = target_side.get_mut(&price) {
                        if let Some(existing_size) = level.get_mut(&order_id) {
                            *existing_size = size;
                        }
                    }
                }
                _ => {} // Ignore other actions like 'T' (trade summaries) or 'R' (clear book) for now
            }
        }

        println!("L3 Book ingestion complete.");
        Ok(())
    }
}