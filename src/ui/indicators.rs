use crate::data::Candle;

#[allow(dead_code)]
pub fn calculate_sma(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    if candles.len() < period {
        return vec![None; candles.len()];
    }

    let mut sma = vec![None; period - 1];

    for i in (period - 1)..candles.len() {
        let sum: f64 = candles[(i - period + 1)..=i]
            .iter()
            .filter_map(|c| c.close.parse::<f64>().ok())
            .sum();
        sma.push(Some(sum / period as f64));
    }

    sma
}

#[allow(dead_code)]
pub fn calculate_rsi(candles: &[Candle], period: usize) -> Vec<Option<f64>> {
    if candles.len() < period + 1 {
        return vec![None; candles.len()];
    }

    let mut rsi = vec![None; period];
    let mut gains = Vec::new();
    let mut losses = Vec::new();

    for i in 1..candles.len() {
        let prev_close: f64 = candles[i - 1].close.parse().unwrap_or(0.0);
        let curr_close: f64 = candles[i].close.parse().unwrap_or(0.0);
        let change = curr_close - prev_close;

        if change > 0.0 {
            gains.push(change);
            losses.push(0.0);
        } else {
            gains.push(0.0);
            losses.push(-change);
        }

        if i >= period {
            let avg_gain: f64 = gains[(i - period)..i].iter().sum::<f64>() / period as f64;
            let avg_loss: f64 = losses[(i - period)..i].iter().sum::<f64>() / period as f64;

            if avg_loss == 0.0 {
                rsi.push(Some(100.0));
            } else {
                let rs = avg_gain / avg_loss;
                let rsi_value = 100.0 - (100.0 / (1.0 + rs));
                rsi.push(Some(rsi_value));
            }
        }
    }

    rsi
}
