use fixed::types::I80F48;
use mango_v4::state::{PerpMarket, QUOTE_DECIMALS};

#[derive(Debug, Copy, Clone)]
pub struct ConversionConf {
    base_decimals: u32,
    base_lot_size: i64,
    quote_lot_size: i64,
}

impl From<PerpMarket> for ConversionConf {
    fn from(value: PerpMarket) -> Self {
        ConversionConf {
            base_decimals: value.base_decimals.into(),
            base_lot_size: value.base_lot_size,
            quote_lot_size: value.quote_lot_size,
        }
    }
}

pub fn native_amount_to_lot(lot_conf: ConversionConf, amount: f64) -> i64 {
    // base_decimals=6
    // 0.0001 in 1e6(decimals) = 100 = 1 lot
    let order_size = I80F48::from_num(amount);

    let exact = order_size * I80F48::from_num(10u64.pow(lot_conf.base_decimals))
        / I80F48::from_num(lot_conf.base_lot_size);

    exact.to_num::<f64>().round() as i64
}

pub fn native_amount(lot_conf: ConversionConf, amount: f64) -> u64 {
    native_amount2(lot_conf.base_decimals, amount)
}

pub fn native_amount2(base_decimals: u32, amount: f64) -> u64 {
    let order_size = I80F48::from_num(amount);

    let exact = order_size * I80F48::from_num(10u64.pow(base_decimals));

    exact.to_num::<f64>().round() as u64
}


pub fn quote_amount_to_lot(lot_conf: ConversionConf, amount: f64) -> i64 {
    // quote_decimals always 6
    let order_size = I80F48::from_num(amount);

    let exact = order_size * I80F48::from_num(10u64.pow(QUOTE_DECIMALS as u32))
        / I80F48::from_num(lot_conf.quote_lot_size);

    exact.to_num::<f64>().round() as i64
}


// base
pub fn quantity_to_lot(lot_conf: ConversionConf, amount: f64) -> I80F48 {
    // base_decimals=6
    // 0.0001 in 1e6(decimals) = 100 = 1 lot
    let order_size = I80F48::from_num(amount);

    order_size * I80F48::from_num(10u64.pow(lot_conf.base_decimals))
        / I80F48::from_num(lot_conf.base_lot_size)
}

mod test {
    use crate::numerics::{ConversionConf, native_amount, native_amount_to_lot, quantity_to_lot, quote_amount_to_lot};

    #[test]
    fn convert_quantity_eth_perp() {


        let sample = ConversionConf {
            base_decimals: 6,
            base_lot_size: 100,
            quote_lot_size: 10,
        };
        
        assert_eq!(1, native_amount_to_lot(sample.clone(), 0.0001));
        assert_eq!(100, native_amount(sample.clone(), 0.0001));
        assert_eq!(10, quote_amount_to_lot(sample.clone(), 0.0001));
        assert_eq!(500 * 1_000_000 / 100, quantity_to_lot(sample.clone(), 500.00));

        // quantity_to_lot()

    }
}

