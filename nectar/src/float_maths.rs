use anyhow::Context;
use bitcoin::hashes::core::cmp::Ordering;
use num::{BigUint, Zero};
use std::str::FromStr;

/// Truncate the float's mantissa to length `precision`.
pub fn truncate(float: f64, precision: u16) -> f64 {
    let mut string = float.to_string();
    let index = string.find('.');

    match index {
        None => float,
        Some(index) => {
            let trunc = index + 1 + precision as usize;
            string.truncate(trunc);
            f64::from_str(&string).expect("This should still be a number")
        }
    }
}

/// Multiply float by 10e`pow`, Returns as a BigUint. No data loss.
/// Errors if the float is negative.
/// Errors if the result is a fraction.
pub fn multiply_pow_ten(float: &str, pow: u16) -> anyhow::Result<BigUint> {
    {
        // Verify that the input is actually a number
        let str = float.replace('.', &"");
        let _ = BigUint::from_str(&str).context("Expecting a float")?;
    }

    let mut float = float.replace('_', &"");
    let decimal_index = float.find('.');

    match decimal_index {
        None => {
            let zeroes = "0".repeat(pow as usize);
            Ok(BigUint::from_str(&format!("{}{}", float, zeroes)).expect("an integer"))
        }
        Some(decimal_index) => {
            let mantissa = float.split_off(decimal_index + 1);
            // Removes the decimal point
            float.truncate(float.len() - 1);
            let integer = float;

            let pow = pow as usize;
            match mantissa.len().cmp(&pow) {
                Ordering::Less => {
                    let remain = pow as usize - mantissa.len();
                    let zeroes = "0".repeat(remain);
                    Ok(
                        BigUint::from_str(&format!("{}{}{}", integer, mantissa, zeroes))
                            .expect("an integer"),
                    )
                }
                Ordering::Equal => {
                    Ok(BigUint::from_str(&format!("{}{}", integer, mantissa)).expect("an integer"))
                }
                Ordering::Greater => anyhow::bail!("Result is not an integer"),
            }
        }
    }
}

/// Divide BigUint by 10e`inv_pow`, Returns as a BigUint.
/// Result is truncated
pub fn divide_pow_ten_trunc(uint: BigUint, inv_pow: usize) -> BigUint {
    let mut uint_str = uint.to_string();

    match uint_str.len().cmp(&inv_pow) {
        Ordering::Less => BigUint::zero(),
        Ordering::Equal => BigUint::zero(),
        Ordering::Greater => {
            uint_str.truncate(uint_str.len() - inv_pow);
            BigUint::from_str(&uint_str).expect("still an integer")
        }
    }
}

pub fn string_int_to_float(int: String, precision: usize) -> String {
    let mut str = int;

    let str = if str.len() <= precision {
        // Need to add "0." in front and some zeros
        let mut prefix = String::from("0.");
        let number_of_zeros = precision - str.len();
        let zeros = "0".repeat(number_of_zeros);
        prefix.push_str(&zeros);
        prefix.push_str(&str);
        prefix
    } else {
        // Need to put a decimal point somewhere
        str.insert(str.len() - precision, '.');
        str
    };

    let str = str.trim_end_matches('0');
    let str = str.trim_end_matches('.');

    if !str.is_empty() {
        str.to_string()
    } else {
        "0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn it_truncates() {
        let float = 1.123_456_789;

        assert_eq!(&truncate(float, 5).to_string(), "1.12345");
    }

    proptest! {
        #[test]
        fn truncate_doesnt_panic(f in any::<f64>(), p in any::<u16>()) {
            truncate(f, p);
        }
    }

    #[test]
    fn given_integer_then_it_multiplies() {
        let float = "123_456_789.0";
        let pow = 6;

        assert_eq!(
            multiply_pow_ten(float, pow).unwrap(),
            BigUint::from(123_456_789_000_000u64)
        )
    }

    #[test]
    fn given_mantissa_of_pow_length_then_it_multiplies() {
        let float = "123.123_456_789";
        let pow = 9;

        assert_eq!(
            multiply_pow_ten(float, pow).unwrap(),
            BigUint::from(123_123_456_789u64)
        )
    }

    #[test]
    fn given_mantissa_length_lesser_than_pow_then_it_multiplies() {
        let float = "123.123_456_789";
        let pow = 12;

        assert_eq!(
            multiply_pow_ten(float, pow).unwrap(),
            BigUint::from(123_123_456_789_000u64)
        )
    }

    #[test]
    fn given_mantissa_length_greater_than_pow_then_it_errors() {
        let float = "123.123_456_789";
        let pow = 6;

        assert!(multiply_pow_ten(float, pow).is_err(),)
    }

    #[test]
    fn given_negative_float_then_it_errors() {
        let float = "-123_456_789.0";
        let pow = 6;

        assert!(multiply_pow_ten(float, pow).is_err(),)
    }

    proptest! {
        #[test]
        fn multiple_pow_ten_doesnt_panic(s in any::<String>(), p in any::<u16>()) {
            let _ = multiply_pow_ten(&s, p);
        }
    }

    #[test]
    fn given_too_precise_uint_it_truncates() {
        let uint = BigUint::from(1_000_000_001u64);
        let pow = 6;
        assert_eq!(divide_pow_ten_trunc(uint, pow), BigUint::from(1_000u64))
    }

    #[test]
    fn given_not_that_precise_uint_it_doesnt_truncate() {
        let uint = BigUint::from(1_234_000_000u64);
        let pow = 6;
        assert_eq!(divide_pow_ten_trunc(uint, pow), BigUint::from(1_234u64))
    }

    #[test]
    fn given_pow_zero_it_doesnt_modifies() {
        let uint = BigUint::from(1_234_567_890u64);
        let pow = 0;
        assert_eq!(divide_pow_ten_trunc(uint.clone(), pow), uint)
    }

    #[test]
    fn given_pow_greater_than_uint_it_truncates_to_zero_1() {
        let uint = BigUint::from(1_234_567_890u64);
        let pow = 10;
        assert_eq!(divide_pow_ten_trunc(uint, pow), BigUint::zero())
    }

    #[test]
    fn given_pow_greater_than_uint_it_truncates_to_zero_2() {
        let uint = BigUint::from(1_234_456_789u64);
        let pow = 11;
        assert_eq!(divide_pow_ten_trunc(uint, pow), BigUint::zero())
    }

    prop_compose! {
        fn new_biguint()(s in "[0-9]+") -> anyhow::Result<BigUint> {
            Ok(BigUint::from_str(&s)?)
        }
    }

    proptest! {
        #[test]
        fn divide_pow_ten_trunc_doesnt_panic(uint in new_biguint(), p in any::<usize>()) {
            if let Ok(uint) = uint {
                let _ = divide_pow_ten_trunc(uint, p);
            }
        }
    }
}
