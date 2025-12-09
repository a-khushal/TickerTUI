use crate::data::Candle;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Chart {
    pub candles: Vec<Candle>,
    pub width: u32,
    pub height: u32,
}

#[allow(dead_code)]
impl Chart {
    pub fn new(candles: Vec<Candle>, width: u32, height: u32) -> Self {
        Self { candles, width, height }
    }

}
