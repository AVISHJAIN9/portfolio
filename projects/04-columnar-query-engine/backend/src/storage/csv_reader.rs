//! CSV Reader with RFC 4180 Quoting, Embedded Newlines & Type Inference (04-058)

#[derive(Debug, Clone, PartialEq)]
pub enum CsvValue {
    Integer(i64),
    Float(f64),
    String(String),
    Null,
}

#[derive(Debug, Clone)]
pub struct CsvTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<CsvValue>>,
}

pub struct CsvReader;

impl CsvReader {
    pub fn parse(input: &str) -> Result<CsvTable, String> {
        let mut lines = Vec::new();
        let mut current_field = String::new();
        let mut current_row = Vec::new();
        let mut in_quotes = false;
        let mut line_num = 1;

        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if in_quotes {
                if c == '"' {
                    if i + 1 < chars.len() && chars[i + 1] == '"' {
                        current_field.push('"');
                        i += 1;
                    } else {
                        in_quotes = false;
                    }
                } else {
                    current_field.push(c);
                    if c == '\n' {
                        line_num += 1;
                    }
                }
            } else {
                match c {
                    '"' => in_quotes = true,
                    ',' => {
                        current_row.push(current_field.trim().to_string());
                        current_field.clear();
                    }
                    '\n' | '\r' => {
                        if c == '\r' && i + 1 < chars.len() && chars[i + 1] == '\n' {
                            i += 1;
                        }
                        current_row.push(current_field.trim().to_string());
                        current_field.clear();
                        if !current_row.is_empty()
                            && !(current_row.len() == 1 && current_row[0].is_empty())
                        {
                            lines.push(current_row);
                        }
                        current_row = Vec::new();
                        line_num += 1;
                    }
                    _ => current_field.push(c),
                }
            }
            i += 1;
        }

        if in_quotes {
            return Err(format!(
                "RFC 4180 parse error: unclosed quote at line {}",
                line_num
            ));
        }

        if !current_field.is_empty() || !current_row.is_empty() {
            current_row.push(current_field.trim().to_string());
            lines.push(current_row);
        }

        if lines.is_empty() {
            return Ok(CsvTable {
                headers: Vec::new(),
                rows: Vec::new(),
            });
        }

        let headers = lines[0].clone();
        let mut typed_rows = Vec::with_capacity(lines.len() - 1);

        for (row_idx, row) in lines[1..].iter().enumerate() {
            if row.len() != headers.len() {
                return Err(format!(
                    "CSV schema error at line {}: expected {} fields, found {}",
                    row_idx + 2,
                    headers.len(),
                    row.len()
                ));
            }
            let typed_row: Vec<CsvValue> = row.iter().map(|val| Self::infer_type(val)).collect();
            typed_rows.push(typed_row);
        }

        Ok(CsvTable {
            headers,
            rows: typed_rows,
        })
    }

    fn infer_type(s: &str) -> CsvValue {
        if s.is_empty() || s.eq_ignore_ascii_case("null") {
            CsvValue::Null
        } else if let Ok(i) = s.parse::<i64>() {
            CsvValue::Integer(i)
        } else if let Ok(f) = s.parse::<f64>() {
            CsvValue::Float(f)
        } else {
            CsvValue::String(s.to_string())
        }
    }
}
