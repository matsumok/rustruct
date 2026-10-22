//! `#rrggbb` の色と、OKLab での間の色。帯の段を 12 色の間から取るときに使う。

use crate::Error;

/// `#rrggbb` → sRGB（0〜1）。
pub fn parse_hex(s: &str) -> Result<[f64; 3], Error> {
    let digits = s
        .strip_prefix('#')
        .ok_or_else(|| Error::NotHex(s.to_string()))?;
    if digits.len() != 6 {
        return Err(Error::NotHex(s.to_string()));
    }
    let channel = |k: usize| {
        u8::from_str_radix(&digits[k..k + 2], 16)
            .map(|v| v as f64 / 255.0)
            .map_err(|_| Error::NotHex(s.to_string()))
    };
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

/// sRGB（0〜1）→ `#rrggbb`。
pub fn to_hex(rgb: [f64; 3]) -> String {
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgb[0]),
        byte(rgb[1]),
        byte(rgb[2])
    )
}

/// `a` から `b` へ OKLab で `t`（0 で `a`、1 で `b`）だけ進んだ色。
pub fn blend(a: &str, b: &str, t: f64) -> Result<String, Error> {
    let (la, lb) = (oklab(parse_hex(a)?), oklab(parse_hex(b)?));
    let t = t.clamp(0.0, 1.0);
    let mixed = [0, 1, 2].map(|k| la[k] + t * (lb[k] - la[k]));
    Ok(to_hex(from_oklab(mixed)))
}

fn to_linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn to_srgb(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn oklab(rgb: [f64; 3]) -> [f64; 3] {
    let [r, g, b] = rgb.map(to_linear);
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    ]
}

fn from_oklab(lab: [f64; 3]) -> [f64; 3] {
    let l = (lab[0] + 0.396_337_777_4 * lab[1] + 0.215_803_757_3 * lab[2]).powi(3);
    let m = (lab[0] - 0.105_561_345_8 * lab[1] - 0.063_854_172_8 * lab[2]).powi(3);
    let s = (lab[0] - 0.089_484_177_5 * lab[1] - 1.291_485_548_0 * lab[2]).powi(3);
    [
        to_srgb(4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s),
        to_srgb(-1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s),
        to_srgb(-0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        for s in ["#000000", "#ffffff", "#3e63dd", "#1c2024"] {
            assert_eq!(to_hex(parse_hex(s).unwrap()), s);
        }
        assert_eq!(parse_hex("3e63dd"), Err(Error::NotHex("3e63dd".into())));
        assert_eq!(parse_hex("#3e63d"), Err(Error::NotHex("#3e63d".into())));
        assert_eq!(parse_hex("#3e63dg"), Err(Error::NotHex("#3e63dg".into())));
    }

    #[test]
    fn blend_ends_are_the_inputs_and_the_middle_is_between() {
        assert_eq!(blend("#1f2d5c", "#f7f9ff", 0.0).unwrap(), "#1f2d5c");
        assert_eq!(blend("#1f2d5c", "#f7f9ff", 1.0).unwrap(), "#f7f9ff");
        // OKLab の明るさの中間（L = 0.5）は、線形で 0.125、sRGB で 0x63 の灰
        assert_eq!(blend("#000000", "#ffffff", 0.5).unwrap(), "#636363");
        let quarter = parse_hex(&blend("#000000", "#ffffff", 0.25).unwrap()).unwrap();
        assert!(quarter[0] > 0.0 && quarter[0] < 0.388, "{quarter:?}");
    }
}
