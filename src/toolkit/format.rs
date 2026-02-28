use egui::NumExt;

pub fn format_with_decimals_in_range(
    value: f64,
    decimal_range: std::ops::RangeInclusive<usize>,
) -> String {
    fn format_with_decimals(value: f64, decimals: usize) -> String {
        FloatFormatOptions::DEFAULT_f64
            .with_decimals(decimals)
            .with_strip_trailing_zeros(false)
            .format(value)
    }

    let epsilon = 16.0 * f32::EPSILON; // margin large enough to handle most peoples round-tripping needs

    let min_decimals = *decimal_range.start();
    let max_decimals = *decimal_range.end();
    debug_assert!(min_decimals <= max_decimals);
    debug_assert!(max_decimals < 100);
    let max_decimals = max_decimals.at_most(16);
    let min_decimals = min_decimals.at_most(max_decimals);

    if min_decimals < max_decimals {
        // Try using a few decimals as possible, and then add more until we have enough precision
        // to round-trip the number.
        for decimals in min_decimals..max_decimals {
            let text = format_with_decimals(value, decimals);
            if let Some(parsed) = parse_f64(&text)
                && egui::emath::almost_equal(parsed as f32, value as f32, epsilon)
            {
                // Enough precision to show the value accurately - good!
                return text;
            }
        }
        // The value has more precision than we expected.
        // Probably the value was set not by the slider, but from outside.
        // In any case: show the full value
    }

    // Use max decimals
    format_with_decimals(value, max_decimals)
}

/// Options for how to format a floating point number, e.g. an [`f64`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FloatFormatOptions {
    /// Always show the sign, even if it is positive (`+`).
    pub always_sign: bool,

    /// Maximum digits of precision to use.
    ///
    /// This includes both the integer part and the fractional part.
    pub precision: usize,

    /// Max number of decimals to show after the decimal point.
    ///
    /// If not specified, [`Self::precision`] is used instead.
    pub num_decimals: Option<usize>,

    pub strip_trailing_zeros: bool,

    /// Only add thousands separators to decimals if there are at least this many decimals.
    pub min_decimals_for_thousands_separators: usize,
}

impl FloatFormatOptions {
    /// Default options for formatting an [`half::f16`].
    #[expect(non_upper_case_globals)]
    pub const DEFAULT_f16: Self = Self {
        always_sign: false,
        precision: 5,
        num_decimals: None,
        strip_trailing_zeros: true,
        min_decimals_for_thousands_separators: 6,
    };

    /// Default options for formatting an [`f32`].
    #[expect(non_upper_case_globals)]
    pub const DEFAULT_f32: Self = Self {
        always_sign: false,
        precision: 7,
        num_decimals: None,
        strip_trailing_zeros: true,
        min_decimals_for_thousands_separators: 6,
    };

    /// Default options for formatting an [`f64`].
    #[expect(non_upper_case_globals)]
    pub const DEFAULT_f64: Self = Self {
        always_sign: false,
        precision: 15,
        num_decimals: None,
        strip_trailing_zeros: true,
        min_decimals_for_thousands_separators: 6,
    };

    /// Always show the sign, even if it is positive (`+`).
    #[inline]
    pub fn with_always_sign(mut self, always_sign: bool) -> Self {
        self.always_sign = always_sign;
        self
    }

    /// Show at most this many digits of precision,
    /// including both the integer part and the fractional part.
    #[inline]
    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    /// Max number of decimals to show after the decimal point.
    ///
    /// If not specified, [`Self::precision`] is used instead.
    #[inline]
    pub fn with_decimals(mut self, num_decimals: usize) -> Self {
        self.num_decimals = Some(num_decimals);
        self
    }

    /// Strip trailing zeros from decimal expansion?
    #[inline]
    pub fn with_strip_trailing_zeros(mut self, strip_trailing_zeros: bool) -> Self {
        self.strip_trailing_zeros = strip_trailing_zeros;
        self
    }

    /// The returned value is for human eyes only, and can not be parsed
    /// by the normal `f64::from_str` function.
    pub fn format(&self, value: impl Into<f64>) -> String {
        self.format_f64(value.into())
    }

    fn format_f64(&self, mut value: f64) -> String {
        fn reverse(s: &str) -> String {
            s.chars().rev().collect()
        }

        let Self {
            always_sign,
            precision,
            num_decimals,
            strip_trailing_zeros,
            min_decimals_for_thousands_separators,
        } = *self;

        if value.is_nan() {
            return "NaN".to_owned();
        }

        let sign = if value < 0.0 {
            value = -value;
            "−" // NOTE: the minus character: <https://www.compart.com/en/unicode/U+2212>
        } else if always_sign {
            "+"
        } else {
            ""
        };

        let abs_string = if value == f64::INFINITY {
            "∞".to_owned()
        } else {
            let magnitude = value.log10();
            let max_decimals = precision as f64 - magnitude.max(0.0);

            if max_decimals < 0.0 {
                // A very large number (more digits than we have precision),
                // so use scientific notation.
                // TODO(emilk): nice formatting of scientific notation with thousands separators
                format!("{:.*e}", precision.saturating_sub(1), value)
            } else {
                let max_decimals = max_decimals as usize;

                let num_decimals = if let Some(num_decimals) = num_decimals {
                    num_decimals.min(max_decimals)
                } else {
                    max_decimals
                };

                let mut formatted = format!("{value:.num_decimals$}");

                if strip_trailing_zeros && formatted.contains('.') {
                    while formatted.ends_with('0') {
                        formatted.pop();
                    }
                    if formatted.ends_with('.') {
                        formatted.pop();
                    }
                }

                if let Some(dot) = formatted.find('.') {
                    let integer_part = &formatted[..dot];
                    let fractional_part = &formatted[dot + 1..];
                    // let fractional_part = &fractional_part[..num_decimals.min(fractional_part.len())];

                    let integer_part = add_thousands_separators(integer_part);

                    if fractional_part.len() < min_decimals_for_thousands_separators {
                        format!("{integer_part}.{fractional_part}")
                    } else {
                        // For the fractional part we should start counting thousand separators from the _front_, so we reverse:
                        let fractional_part =
                            reverse(&add_thousands_separators(&reverse(fractional_part)));
                        format!("{integer_part}.{fractional_part}")
                    }
                } else {
                    add_thousands_separators(&formatted) // it's an integer
                }
            }
        };

        format!("{sign}{abs_string}")
    }
}

/// Format a number with about 15 decimals of precision.
///
/// The returned value is for human eyes only, and can not be parsed
/// by the normal `f64::from_str` function.
pub fn format_f64(value: f64) -> String {
    FloatFormatOptions::DEFAULT_f64.format(value)
}

/// Format a number with about 7 decimals of precision.
///
/// The returned value is for human eyes only, and can not be parsed
/// by the normal `f64::from_str` function.
pub fn format_f32(value: f32) -> String {
    FloatFormatOptions::DEFAULT_f32.format(value)
}

/// Format a latitude or longitude value.
///
/// For human eyes only.
pub fn format_lat_lon(value: f64) -> String {
    format!(
        "{}°",
        FloatFormatOptions {
            always_sign: true,
            precision: 10,
            num_decimals: Some(6),
            strip_trailing_zeros: false,
            min_decimals_for_thousands_separators: 10,
        }
        .format_f64(value)
    )
}

// --- Numbers ---

/// The minus character: <https://www.compart.com/en/unicode/U+2212>
///
/// Looks slightly different from the normal hyphen `-`.
pub const MINUS: char = '−';

/// A thin space, used for thousands separators, like `1 234`:
///
/// <https://en.wikipedia.org/wiki/Thin_space>
pub const THIN_SPACE: char = '\u{2009}';

/// Prepare a string containing a number for parsing
pub fn strip_whitespace_and_normalize(text: &str) -> String {
    text.chars()
        // Ignore whitespace (trailing, leading, and thousands separators):
        .filter(|c| !c.is_whitespace())
        // Replace special minus character with normal minus (hyphen):
        .map(|c| if c == MINUS { '-' } else { c })
        .collect()
}

/// Add thousands separators to a number, every three steps,
/// counting from the last character.
fn add_thousands_separators(number: &str) -> String {
    let mut chars = number.chars().rev().peekable();

    let mut result = vec![];
    while chars.peek().is_some() {
        if !result.is_empty() {
            // thousands-deliminator:
            result.push(THIN_SPACE);
        }
        for _ in 0..3 {
            if let Some(c) = chars.next() {
                result.push(c);
            }
        }
    }

    result.reverse();
    result.into_iter().collect()
}
/// Parse
/// s a number, ignoring whitespace (e.g. thousand separators),
/// and treating the special minus character `MINUS` (−) as a minus sign.
pub fn parse_f64(text: &str) -> Option<f64> {
    let text = strip_whitespace_and_normalize(text);
    text.parse().ok()
}
