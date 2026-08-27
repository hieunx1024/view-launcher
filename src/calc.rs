use std::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Ultra-fast, zero-dependency inline arithmetic expression evaluator for view-launcher.
/// Supports +, -, *, /, %, ^, parentheses, floats, integers, hex (0x...), and bin (0b...).

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,
    LParen,
    RParen,
    Sqrt,
    Abs,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    has_operator: bool,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            has_operator: false,
        }
    }

    fn tokenize(&mut self) -> Option<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(&c) = self.chars.peek() {
            match c {
                ' ' | '\t' | '\r' | '\n' => {
                    self.chars.next();
                }
                '+' => {
                    tokens.push(Token::Plus);
                    self.has_operator = true;
                    self.chars.next();
                }
                '-' => {
                    tokens.push(Token::Minus);
                    self.has_operator = true;
                    self.chars.next();
                }
                '*' | 'x' | 'X' | '×' => {
                    tokens.push(Token::Multiply);
                    self.has_operator = true;
                    self.chars.next();
                }
                '/' | '÷' => {
                    tokens.push(Token::Divide);
                    self.has_operator = true;
                    self.chars.next();
                }
                '%' => {
                    tokens.push(Token::Modulo);
                    self.has_operator = true;
                    self.chars.next();
                }
                '^' => {
                    tokens.push(Token::Power);
                    self.has_operator = true;
                    self.chars.next();
                }
                '(' => {
                    tokens.push(Token::LParen);
                    self.chars.next();
                }
                ')' => {
                    tokens.push(Token::RParen);
                    self.chars.next();
                }
                '0'..='9' | '.' => {
                    let num = self.read_number()?;
                    tokens.push(Token::Number(num));
                }
                'a'..='z' | 'A'..='Z' => {
                    let ident = self.read_ident();
                    match ident.to_lowercase().as_str() {
                        "sqrt" => {
                            tokens.push(Token::Sqrt);
                            self.has_operator = true;
                        }
                        "abs" => {
                            tokens.push(Token::Abs);
                            self.has_operator = true;
                        }
                        "pi" => {
                            tokens.push(Token::Number(std::f64::consts::PI));
                        }
                        "e" => {
                            tokens.push(Token::Number(std::f64::consts::E));
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            }
        }

        // Must contain at least one operator or math function to be treated as a calculator query
        if !self.has_operator || tokens.is_empty() {
            return None;
        }

        Some(tokens)
    }

    fn read_number(&mut self) -> Option<f64> {
        let mut s = String::new();
        
        // Check for 0x (hex) or 0b (bin)
        if let Some(&'0') = self.chars.peek() {
            s.push(self.chars.next().unwrap());
            if let Some(&c) = self.chars.peek() {
                if c == 'x' || c == 'X' {
                    s.clear();
                    self.chars.next(); // consume x
                    while let Some(&h) = self.chars.peek() {
                        if h.is_ascii_hexdigit() {
                            s.push(self.chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() { return None; }
                    return i64::from_str_radix(&s, 16).ok().map(|v| v as f64);
                } else if c == 'b' || c == 'B' {
                    s.clear();
                    self.chars.next(); // consume b
                    while let Some(&b) = self.chars.peek() {
                        if b == '0' || b == '1' {
                            s.push(self.chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    if s.is_empty() { return None; }
                    return i64::from_str_radix(&s, 2).ok().map(|v| v as f64);
                }
            }
        }

        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }

        s.parse::<f64>().ok()
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_alphabetic() {
                s.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }
        s
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse(&mut self) -> Option<f64> {
        let result = self.parse_expr()?;
        if self.pos == self.tokens.len() {
            Some(result)
        } else {
            None
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&Token> {
        let tok = self.tokens.get(self.pos);
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self) -> Option<f64> {
        let mut left = self.parse_term()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    let right = self.parse_term()?;
                    left += right;
                }
                Token::Minus => {
                    self.next();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }

        Some(left)
    }

    fn parse_term(&mut self) -> Option<f64> {
        let mut left = self.parse_power()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Multiply => {
                    self.next();
                    let right = self.parse_power()?;
                    left *= right;
                }
                Token::Divide => {
                    self.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None; // Division by zero
                    }
                    left /= right;
                }
                Token::Modulo => {
                    self.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return None;
                    }
                    left %= right;
                }
                _ => break,
            }
        }

        Some(left)
    }

    fn parse_power(&mut self) -> Option<f64> {
        let left = self.parse_factor()?;

        if let Some(Token::Power) = self.peek() {
            self.next();
            let right = self.parse_power()?;
            Some(left.powf(right))
        } else {
            Some(left)
        }
    }

    fn parse_factor(&mut self) -> Option<f64> {
        match self.peek()? {
            Token::Number(val) => {
                let v = *val;
                self.next();
                Some(v)
            }
            Token::Minus => {
                self.next();
                let val = self.parse_factor()?;
                Some(-val)
            }
            Token::Plus => {
                self.next();
                self.parse_factor()
            }
            Token::LParen => {
                self.next();
                let val = self.parse_expr()?;
                if let Some(Token::RParen) = self.next() {
                    Some(val)
                } else {
                    None
                }
            }
            Token::Sqrt => {
                self.next();
                let val = self.parse_factor()?;
                if val < 0.0 {
                    None
                } else {
                    Some(val.sqrt())
                }
            }
            Token::Abs => {
                self.next();
                let val = self.parse_factor()?;
                Some(val.abs())
            }
            _ => None,
        }
    }
}

/// Evaluates a user query string as an arithmetic expression.
/// Returns `Some(f64)` if it is a valid mathematical expression, otherwise `None`.
pub fn evaluate(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut lexer = Lexer::new(trimmed);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let result = parser.parse()?;

    if result.is_finite() {
        Some(result)
    } else {
        None
    }
}

/// Formats a float calculation result cleanly with thousand separators for both integer and decimal numbers.
pub fn format_result(val: f64) -> String {
    format_number_with_separators(val, 6)
}

/// Formats any float with thousand separators on the integer part and trimmed decimals.
pub fn format_number_with_separators(val: f64, max_decimals: usize) -> String {
    if !val.is_finite() {
        return val.to_string();
    }

    let is_negative = val < 0.0;
    let abs_val = val.abs();

    if abs_val.fract() == 0.0 && abs_val < 1e15 {
        let int_val = abs_val as u64;
        let formatted = format_integer_separators(int_val);
        if is_negative {
            format!("-{}", formatted)
        } else {
            formatted
        }
    } else {
        let rounded = format!("{:.1$}", abs_val, max_decimals);
        let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
        
        let parts: Vec<&str> = trimmed.split('.').collect();
        let int_part: u64 = parts[0].parse().unwrap_or(0);
        let int_formatted = format_integer_separators(int_part);

        let final_str = if parts.len() > 1 && !parts[1].is_empty() {
            format!("{}.{}", int_formatted, parts[1])
        } else {
            int_formatted
        };

        if is_negative {
            format!("-{}", final_str)
        } else {
            final_str
        }
    }
}

fn format_integer_separators(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut s = String::new();
    let mut count = 0;
    while n > 0 {
        if count > 0 && count % 3 == 0 {
            s.push(',');
        }
        s.push(char::from_digit((n % 10) as u32, 10).unwrap());
        n /= 10;
        count += 1;
    }
    s.chars().rev().collect()
}

/// Formats currency values following standard financial conventions:
/// - VND, JPY, KRW, IDR: No decimals (e.g. "26,062 VND")
/// - USD, EUR, GBP, etc.: 2 decimals (e.g. "98.23 USD")
pub fn format_currency(val: f64, currency_code: &str) -> String {
    let code = currency_code.to_uppercase();
    match code.as_str() {
        "VND" | "JPY" | "KRW" | "IDR" | "CLP" | "HUF" | "TWD" => {
            let rounded = val.round();
            format_number_with_separators(rounded, 0)
        }
        _ => {
            if val.fract() == 0.0 {
                format_number_with_separators(val, 0)
            } else {
                format_number_with_separators(val, 2)
            }
        }
    }
}

/// Result of a smart unit or currency conversion.
#[derive(Debug, PartialEq, Clone)]
pub struct ConversionResult {
    pub title: String,
    pub subtitle: String,
    pub value_to_copy: String,
}

/// Evaluates smart unit conversions and currency conversions offline.
/// Examples:
/// - "100 usd in vnd" -> "2,545,000 VND"
/// - "50 km to mi" -> "31.07 mi"
/// - "37 c to f" -> "98.6 °F"
/// - "1024 mb in gb" -> "1 GB"
/// - "72 hours in days" -> "3 days"
pub fn evaluate_conversion(query: &str) -> Option<ConversionResult> {
    let lower = query.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }

    // Must contain conversion keywords or 2 distinct unit tokens
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    // Pattern 1: [number, from_unit, "in"|"to"|"=", to_unit]
    // Pattern 2: [number, from_unit, to_unit]
    // Pattern 3: [number_with_from_unit, "in"|"to"|"=", to_unit]
    let (val, from_unit, to_unit) = parse_conversion_query(&tokens, &lower)?;

    // 1. Try Temperature Conversion
    if let Some((res_val, res_label, full_label)) = convert_temperature(val, &from_unit, &to_unit) {
        let formatted = format_result(res_val);
        let title = format!("{} {}", formatted, res_label);
        let subtitle = format!("{} {} = {} (Press Enter to copy)", format_result(val), full_label.0, title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    // 2. Try Length Conversion
    if let Some((res_val, res_label, from_label)) = convert_units_by_table(val, &from_unit, &to_unit, &LENGTH_TABLE) {
        let formatted = format_result(res_val);
        let title = format!("{} {}", formatted, res_label);
        let subtitle = format!("{} {} = {} (Press Enter to copy)", format_result(val), from_label, title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    // 3. Try Weight/Mass Conversion
    if let Some((res_val, res_label, from_label)) = convert_units_by_table(val, &from_unit, &to_unit, &WEIGHT_TABLE) {
        let formatted = format_result(res_val);
        let title = format!("{} {}", formatted, res_label);
        let subtitle = format!("{} {} = {} (Press Enter to copy)", format_result(val), from_label, title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    // 4. Try Digital Storage Conversion
    if let Some((res_val, res_label, from_label)) = convert_units_by_table(val, &from_unit, &to_unit, &STORAGE_TABLE) {
        let formatted = format_result(res_val);
        let title = format!("{} {}", formatted, res_label);
        let subtitle = format!("{} {} = {} (Press Enter to copy)", format_result(val), from_label, title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    // 5. Try Time Conversion
    if let Some((res_val, res_label, from_label)) = convert_units_by_table(val, &from_unit, &to_unit, &TIME_TABLE) {
        let formatted = format_result(res_val);
        let title = format!("{} {}", formatted, res_label);
        let subtitle = format!("{} {} = {} (Press Enter to copy)", format_result(val), from_label, title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    // 6. Try Speed Conversion
    if let Some((res_val, res_label, from_label)) = convert_units_by_table(val, &from_unit, &to_unit, &SPEED_TABLE) {
        let formatted = format_result(res_val);
        let title = format!("{} {}", formatted, res_label);
        let subtitle = format!("{} {} = {} (Press Enter to copy)", format_result(val), from_label, title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    // 7. Try Currency Conversion (Dynamic live rates or offline fallback)
    if let Some((res_val, res_label, from_label)) = convert_currency_dynamic(val, &from_unit, &to_unit) {
        let formatted = format_currency(res_val, &res_label);
        let val_formatted = format_currency(val, &from_label);
        let title = format!("{} {}", formatted, res_label.to_uppercase());
        let subtitle = format!("{} {} = {} (Press Enter to copy)", val_formatted, from_label.to_uppercase(), title);
        return Some(ConversionResult {
            title,
            subtitle,
            value_to_copy: formatted,
        });
    }

    None
}

static DYNAMIC_RATES: RwLock<Option<HashMap<String, f64>>> = RwLock::new(None);

fn get_rates_cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("view-launcher");
        p.push("rates.json");
        p
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RatesCache {
    timestamp: u64,
    rates: HashMap<String, f64>,
}

/// Initializes local currency rates from disk cache and triggers a background 24h refresh.
pub fn init_currency_rates() {
    // 1. Load local cache immediately (< 0.1ms)
    if let Some(path) = get_rates_cache_path() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cache) = serde_json::from_str::<RatesCache>(&content) {
                if let Ok(mut lock) = DYNAMIC_RATES.write() {
                    *lock = Some(cache.rates);
                }
            }
        }
    }

    // 2. Check if cache is older than 24h (86400s) or missing, and refresh in background
    std::thread::spawn(move || {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let need_refresh = if let Some(path) = get_rates_cache_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cache) = serde_json::from_str::<RatesCache>(&content) {
                    now.saturating_sub(cache.timestamp) > 86400
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if need_refresh {
            fetch_and_save_rates(now);
        }
    });
}

fn fetch_and_save_rates(now: u64) {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(&["-s", "--max-time", "3", "https://open.er-api.com/v6/latest/USD"]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(rates_obj) = json.get("rates").and_then(|r| r.as_object()) {
                    let mut rates_map = HashMap::new();
                    for (k, v) in rates_obj {
                        if let Some(num) = v.as_f64() {
                            rates_map.insert(k.to_uppercase(), num);
                        }
                    }

                    if !rates_map.is_empty() {
                        let cache = RatesCache {
                            timestamp: now,
                            rates: rates_map.clone(),
                        };

                        if let Some(path) = get_rates_cache_path() {
                            if let Some(parent) = path.parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            if let Ok(serialized) = serde_json::to_string(&cache) {
                                let _ = fs::write(path, serialized);
                            }
                        }

                        if let Ok(mut lock) = DYNAMIC_RATES.write() {
                            *lock = Some(rates_map);
                        }
                    }
                }
            }
        }
    }
}

fn normalize_currency_code(alias: &str) -> Option<&'static str> {
    match alias {
        "usd" | "$" | "dollar" | "dollars" | "bucks" => Some("USD"),
        "vnd" | "đ" | "dong" | "đồng" | "vnđ" => Some("VND"),
        "eur" | "€" | "euro" | "euros" => Some("EUR"),
        "jpy" | "¥" | "yen" => Some("JPY"),
        "gbp" | "£" | "pound" | "pounds" | "quid" => Some("GBP"),
        "cny" | "rmb" | "yuan" => Some("CNY"),
        "sgd" => Some("SGD"),
        "krw" | "won" => Some("KRW"),
        "thb" | "baht" => Some("THB"),
        "aud" => Some("AUD"),
        "cad" => Some("CAD"),
        "chf" | "franc" => Some("CHF"),
        "inr" | "rupee" => Some("INR"),
        "brl" => Some("BRL"),
        "idr" | "rupiah" => Some("IDR"),
        "myr" | "ringgit" => Some("MYR"),
        "php" | "peso" => Some("PHP"),
        "twd" => Some("TWD"),
        "hkd" => Some("HKD"),
        "nzd" => Some("NZD"),
        "rub" | "ruble" => Some("RUB"),
        other => {
            if other.len() == 3 && other.chars().all(|c| c.is_ascii_alphabetic()) {
                let upper = other.to_uppercase();
                let leaked: &'static str = Box::leak(upper.into_boxed_str());
                Some(leaked)
            } else {
                None
            }
        }
    }
}

fn convert_currency_dynamic(val: f64, from_u: &str, to_u: &str) -> Option<(f64, String, String)> {
    let from_code = normalize_currency_code(from_u)?;
    let to_code = normalize_currency_code(to_u)?;

    // 1. Try dynamic cached rates
    if let Ok(lock) = DYNAMIC_RATES.read() {
        if let Some(ref rates) = *lock {
            let from_rate = if from_code == "USD" { Some(1.0) } else { rates.get(from_code).copied() };
            let to_rate = if to_code == "USD" { Some(1.0) } else { rates.get(to_code).copied() };

            if let (Some(f_rate), Some(t_rate)) = (from_rate, to_rate) {
                let val_in_usd = val / f_rate;
                let res = val_in_usd * t_rate;
                return Some((res, to_code.to_string(), from_code.to_string()));
            }
        }
    }

    // 2. Fallback to offline reference rates
    if let Some((res, to_canon, from_canon)) = convert_units_by_table(val, from_u, to_u, &CURRENCY_TABLE) {
        return Some((res, to_canon.to_string(), from_canon.to_string()));
    }

    None
}

fn parse_conversion_query(tokens: &[&str], _full_lower: &str) -> Option<(f64, String, String)> {
    if tokens.is_empty() {
        return None;
    }

    // Check if first token starts with a number
    let first = tokens[0];
    let mut num_end = 0;
    for (i, c) in first.chars().enumerate() {
        if c.is_ascii_digit() || c == '.' || (i == 0 && c == '-') {
            num_end = i + 1;
        } else {
            break;
        }
    }

    if num_end > 0 {
        let num_part = &first[..num_end];
        let val: f64 = num_part.parse().ok()?;
        let remaining_first = &first[num_end..];

        if !remaining_first.is_empty() {
            // Case "100usd in vnd" or "50km to mi"
            let from_unit = remaining_first.to_string();
            let mut target_idx = 1;
            if target_idx < tokens.len() && (tokens[target_idx] == "in" || tokens[target_idx] == "to" || tokens[target_idx] == "=" || tokens[target_idx] == "->") {
                target_idx += 1;
            }
            if target_idx < tokens.len() {
                let to_unit = tokens[target_idx].to_string();
                return Some((val, from_unit, to_unit));
            }
        } else {
            // Case "100 usd in vnd" or "100 usd vnd"
            if tokens.len() >= 3 {
                let from_unit = tokens[1].to_string();
                let mut target_idx = 2;
                if tokens[target_idx] == "in" || tokens[target_idx] == "to" || tokens[target_idx] == "=" || tokens[target_idx] == "->" {
                    target_idx += 1;
                }
                if target_idx < tokens.len() {
                    let to_unit = tokens[target_idx].to_string();
                    return Some((val, from_unit, to_unit));
                }
            }
        }
    }

    None
}

struct UnitDef {
    aliases: &'static [&'static str],
    factor_to_base: f64,
    canonical_name: &'static str,
}

const LENGTH_TABLE: &[UnitDef] = &[
    UnitDef { aliases: &["km", "kilometer", "kilometers", "kmh"], factor_to_base: 1000.0, canonical_name: "km" },
    UnitDef { aliases: &["m", "meter", "meters", "metre"], factor_to_base: 1.0, canonical_name: "m" },
    UnitDef { aliases: &["cm", "centimeter", "centimeters"], factor_to_base: 0.01, canonical_name: "cm" },
    UnitDef { aliases: &["mm", "millimeter", "millimeters"], factor_to_base: 0.001, canonical_name: "mm" },
    UnitDef { aliases: &["mi", "mile", "miles"], factor_to_base: 1609.344, canonical_name: "mi" },
    UnitDef { aliases: &["yd", "yard", "yards"], factor_to_base: 0.9144, canonical_name: "yd" },
    UnitDef { aliases: &["ft", "feet", "foot"], factor_to_base: 0.3048, canonical_name: "ft" },
    UnitDef { aliases: &["in", "inch", "inches"], factor_to_base: 0.0254, canonical_name: "in" },
];

const WEIGHT_TABLE: &[UnitDef] = &[
    UnitDef { aliases: &["kg", "kilogram", "kilograms", "kilo", "kilos"], factor_to_base: 1000.0, canonical_name: "kg" },
    UnitDef { aliases: &["g", "gram", "grams"], factor_to_base: 1.0, canonical_name: "g" },
    UnitDef { aliases: &["mg", "milligram", "milligrams"], factor_to_base: 0.001, canonical_name: "mg" },
    UnitDef { aliases: &["lb", "lbs", "pound", "pounds"], factor_to_base: 453.59237, canonical_name: "lbs" },
    UnitDef { aliases: &["oz", "ounce", "ounces"], factor_to_base: 28.349523, canonical_name: "oz" },
    UnitDef { aliases: &["ton", "tons", "tonne"], factor_to_base: 1000000.0, canonical_name: "tons" },
];

const STORAGE_TABLE: &[UnitDef] = &[
    UnitDef { aliases: &["b", "byte", "bytes"], factor_to_base: 1.0, canonical_name: "B" },
    UnitDef { aliases: &["kb", "kilobyte", "kilobytes"], factor_to_base: 1024.0, canonical_name: "KB" },
    UnitDef { aliases: &["mb", "megabyte", "megabytes"], factor_to_base: 1024.0 * 1024.0, canonical_name: "MB" },
    UnitDef { aliases: &["gb", "gigabyte", "gigabytes"], factor_to_base: 1024.0 * 1024.0 * 1024.0, canonical_name: "GB" },
    UnitDef { aliases: &["tb", "terabyte", "terabytes"], factor_to_base: 1024.0 * 1024.0 * 1024.0 * 1024.0, canonical_name: "TB" },
    UnitDef { aliases: &["pb", "petabyte"], factor_to_base: 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0, canonical_name: "PB" },
];

const TIME_TABLE: &[UnitDef] = &[
    UnitDef { aliases: &["s", "sec", "secs", "second", "seconds", "giây"], factor_to_base: 1.0, canonical_name: "seconds" },
    UnitDef { aliases: &["m", "min", "mins", "minute", "minutes", "phút"], factor_to_base: 60.0, canonical_name: "minutes" },
    UnitDef { aliases: &["h", "hr", "hrs", "hour", "hours", "giờ"], factor_to_base: 3600.0, canonical_name: "hours" },
    UnitDef { aliases: &["d", "day", "days", "ngày"], factor_to_base: 86400.0, canonical_name: "days" },
    UnitDef { aliases: &["w", "week", "weeks", "tuần"], factor_to_base: 604800.0, canonical_name: "weeks" },
    UnitDef { aliases: &["mo", "month", "months", "tháng"], factor_to_base: 2592000.0, canonical_name: "months" },
    UnitDef { aliases: &["y", "yr", "yrs", "year", "years", "năm"], factor_to_base: 31536000.0, canonical_name: "years" },
];

const SPEED_TABLE: &[UnitDef] = &[
    UnitDef { aliases: &["m/s", "mps"], factor_to_base: 1.0, canonical_name: "m/s" },
    UnitDef { aliases: &["km/h", "kmh", "kph"], factor_to_base: 1.0 / 3.6, canonical_name: "km/h" },
    UnitDef { aliases: &["mph", "mi/h"], factor_to_base: 0.44704, canonical_name: "mph" },
];

// Offline reference rate with base = 1.0 USD
const CURRENCY_TABLE: &[UnitDef] = &[
    UnitDef { aliases: &["usd", "$", "dollar", "bucks"], factor_to_base: 1.0, canonical_name: "USD" },
    UnitDef { aliases: &["vnd", "đ", "dong", "đồng", "vnđ"], factor_to_base: 1.0 / 25450.0, canonical_name: "VND" },
    UnitDef { aliases: &["eur", "€", "euro"], factor_to_base: 1.0 / 0.92, canonical_name: "EUR" },
    UnitDef { aliases: &["jpy", "¥", "yen"], factor_to_base: 1.0 / 152.0, canonical_name: "JPY" },
    UnitDef { aliases: &["gbp", "£", "pound", "quid"], factor_to_base: 1.0 / 0.79, canonical_name: "GBP" },
    UnitDef { aliases: &["cny", "rmb", "yuan"], factor_to_base: 1.0 / 7.24, canonical_name: "CNY" },
    UnitDef { aliases: &["sgd"], factor_to_base: 1.0 / 1.35, canonical_name: "SGD" },
    UnitDef { aliases: &["krw", "won"], factor_to_base: 1.0 / 1380.0, canonical_name: "KRW" },
    UnitDef { aliases: &["thb", "baht"], factor_to_base: 1.0 / 36.5, canonical_name: "THB" },
    UnitDef { aliases: &["aud"], factor_to_base: 1.0 / 1.54, canonical_name: "AUD" },
    UnitDef { aliases: &["cad"], factor_to_base: 1.0 / 1.38, canonical_name: "CAD" },
];

fn convert_units_by_table(val: f64, from_u: &str, to_u: &str, table: &[UnitDef]) -> Option<(f64, &'static str, &'static str)> {
    let from_def = table.iter().find(|u| u.aliases.iter().any(|&a| a == from_u))?;
    let to_def = table.iter().find(|u| u.aliases.iter().any(|&a| a == to_u))?;

    let val_in_base = val * from_def.factor_to_base;
    let res = val_in_base / to_def.factor_to_base;
    Some((res, to_def.canonical_name, from_def.canonical_name))
}

fn convert_temperature(val: f64, from_u: &str, to_u: &str) -> Option<(f64, &'static str, (&'static str, &'static str))> {
    let is_c = |u: &str| u == "c" || u == "celsius" || u == "°c" || u == "do c" || u == "độ c";
    let is_f = |u: &str| u == "f" || u == "fahrenheit" || u == "°f" || u == "do f" || u == "độ f";
    let is_k = |u: &str| u == "k" || u == "kelvin";

    if is_c(from_u) && is_f(to_u) {
        let res = (val * 9.0 / 5.0) + 32.0;
        Some((res, "°F", ("°C", "°F")))
    } else if is_f(from_u) && is_c(to_u) {
        let res = (val - 32.0) * 5.0 / 9.0;
        Some((res, "°C", ("°F", "°C")))
    } else if is_c(from_u) && is_k(to_u) {
        let res = val + 273.15;
        Some((res, "K", ("°C", "K")))
    } else if is_k(from_u) && is_c(to_u) {
        let res = val - 273.15;
        Some((res, "°C", ("K", "°C")))
    } else if is_f(from_u) && is_k(to_u) {
        let res = (val - 32.0) * 5.0 / 9.0 + 273.15;
        Some((res, "K", ("°F", "K")))
    } else if is_k(from_u) && is_f(to_u) {
        let res = (val - 273.15) * 9.0 / 5.0 + 32.0;
        Some((res, "°F", ("K", "°F")))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(evaluate("2 + 2"), Some(4.0));
        assert_eq!(evaluate("10 - 3 * 2"), Some(4.0));
        assert_eq!(evaluate("(10 - 3) * 2"), Some(14.0));
        assert_eq!(evaluate("100 / 4"), Some(25.0));
        assert_eq!(evaluate("2 ^ 10"), Some(1024.0));
        assert_eq!(evaluate("10 % 3"), Some(1.0));
    }

    #[test]
    fn test_hex_and_bin() {
        assert_eq!(evaluate("0x10 + 5"), Some(21.0));
        assert_eq!(evaluate("0b1010 + 2"), Some(12.0));
    }

    #[test]
    fn test_formatting() {
        assert_eq!(format_result(1024.0), "1,024");
        assert_eq!(format_result(1000000.0), "1,000,000");
        assert_eq!(format_result(3.14159), "3.14159");
        assert_eq!(format_result(1234567.89), "1,234,567.89");
        assert_eq!(format_currency(26062.2142, "VND"), "26,062");
        assert_eq!(format_currency(98.2345, "USD"), "98.23");
        assert_eq!(format_currency(1250000.0, "VND"), "1,250,000");
    }

    #[test]
    fn test_unit_conversions() {
        let c1 = evaluate_conversion("50 km to mi").unwrap();
        assert!(c1.title.contains("mi"));

        let c2 = evaluate_conversion("37 c to f").unwrap();
        assert_eq!(c2.title, "98.6 °F");

        let c3 = evaluate_conversion("1024 mb in gb").unwrap();
        assert_eq!(c3.title, "1 GB");

        let c4 = evaluate_conversion("72 hours in days").unwrap();
        assert_eq!(c4.title, "3 days");

        let c5 = evaluate_conversion("100 usd in vnd").unwrap();
        assert!(c5.title.contains("VND"));
    }

    #[test]
    fn test_non_math_queries() {
        assert_eq!(evaluate("firefox"), None);
        assert_eq!(evaluate("123"), None);
        assert_eq!(evaluate("code /home"), None);
    }
}
