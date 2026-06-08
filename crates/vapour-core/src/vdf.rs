//! A minimal text-VDF / KeyValues parser for Steam's on-disk `.acf` / `.vdf` files
//! (`appmanifest_<appid>.acf`, `libraryfolders.vdf`).
//!
//! This is the *text* KeyValues format (`"key" "value"` / `"key" { … }`), not the binary KV
//! used by PICS (that lives in `vapour-protocol::kv`). It is intentionally tiny: enough to read
//! install metadata, no more. Key lookups are case-insensitive (Steam mixes `installdir` and
//! `StateFlags`). Pure → unit-tested against real captured files.

/// A parsed KeyValues node: an ordered list of `(key, value)` pairs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VdfMap {
    entries: Vec<(String, VdfValue)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VdfValue {
    Str(String),
    Map(VdfMap),
}

impl VdfMap {
    /// Case-insensitive lookup of a string-valued key.
    pub fn str(&self, key: &str) -> Option<&str> {
        self.entries.iter().find_map(|(k, v)| match v {
            VdfValue::Str(s) if k.eq_ignore_ascii_case(key) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Case-insensitive lookup of a nested map.
    pub fn map(&self, key: &str) -> Option<&VdfMap> {
        self.entries.iter().find_map(|(k, v)| match v {
            VdfValue::Map(m) if k.eq_ignore_ascii_case(key) => Some(m),
            _ => None,
        })
    }

    /// All `(key, value)` pairs in order — used to iterate the numbered children of
    /// `libraryfolders.vdf`.
    pub fn entries(&self) -> &[(String, VdfValue)] {
        &self.entries
    }
}

/// Parse a text-VDF document into its root map. Returns `None` on malformed input.
pub fn parse(input: &str) -> Option<VdfMap> {
    let mut lexer = Lexer::new(input);
    let mut root = VdfMap::default();
    // A document is a sequence of `key value` pairs at the top level.
    loop {
        match lexer.next_token()? {
            Token::Eof => return Some(root),
            Token::Str(key) => {
                let value = parse_value(&mut lexer)?;
                root.entries.push((key, value));
            }
            // A stray brace at the top level is malformed.
            Token::Open | Token::Close => return None,
        }
    }
}

fn parse_value(lexer: &mut Lexer<'_>) -> Option<VdfValue> {
    match lexer.next_token()? {
        Token::Str(s) => Some(VdfValue::Str(s)),
        Token::Open => parse_map(lexer).map(VdfValue::Map),
        Token::Close | Token::Eof => None,
    }
}

/// Parse the body of a `{ … }` block up to and including the closing brace.
fn parse_map(lexer: &mut Lexer<'_>) -> Option<VdfMap> {
    let mut map = VdfMap::default();
    loop {
        match lexer.next_token()? {
            Token::Close => return Some(map),
            Token::Eof | Token::Open => return None,
            Token::Str(key) => {
                let value = parse_value(lexer)?;
                map.entries.push((key, value));
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Str(String),
    Open,
    Close,
    Eof,
}

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_ws_and_comments();
        let Some(&byte) = self.input.get(self.pos) else {
            return Some(Token::Eof);
        };
        match byte {
            b'{' => {
                self.pos += 1;
                Some(Token::Open)
            }
            b'}' => {
                self.pos += 1;
                Some(Token::Close)
            }
            b'"' => self.read_quoted().map(Token::Str),
            _ => self.read_bare().map(Token::Str),
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self
                .input
                .get(self.pos)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.pos += 1;
            }
            if self.input.get(self.pos..self.pos + 2) == Some(b"//") {
                self.pos += 2;
                while self.input.get(self.pos).is_some_and(|b| *b != b'\n') {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    fn read_quoted(&mut self) -> Option<String> {
        self.pos += 1; // opening quote
        let mut out = Vec::new();
        while let Some(&byte) = self.input.get(self.pos) {
            self.pos += 1;
            match byte {
                b'"' => return Some(String::from_utf8_lossy(&out).into_owned()),
                b'\\' => {
                    let escaped = *self.input.get(self.pos)?;
                    self.pos += 1;
                    out.push(match escaped {
                        b'n' => b'\n',
                        b't' => b'\t',
                        b'r' => b'\r',
                        other => other, // covers \\ and \"
                    });
                }
                other => out.push(other),
            }
        }
        None // unterminated string
    }

    fn read_bare(&mut self) -> Option<String> {
        let start = self.pos;
        while self
            .input
            .get(self.pos)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'{' && *b != b'}')
        {
            self.pos += 1;
        }
        (self.pos > start).then(|| String::from_utf8_lossy(&self.input[start..self.pos]).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A real `appmanifest_1295660.acf` (Civilization VII) head, captured from this machine.
    const CIV_VII_ACF: &str = r#"
"AppState"
{
	"appid"		"1295660"
	"universe"		"1"
	"name"		"Sid Meier's Civilization VII"
	"StateFlags"		"4"
	"installdir"		"Sid Meier's Civilization VII"
	"LastUpdated"		"1780665886"
}
"#;

    // A real `libraryfolders.vdf` (two libraries), captured from this machine.
    const LIBRARYFOLDERS: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps"
		{
			"228980"		"476357342"
		}
	}
	"1"
	{
		"path"		"E:\\Games\\Steam"
		"apps"
		{
			"1295660"		"24369453207"
		}
	}
}
"#;

    #[test]
    fn parses_appmanifest_fields() {
        let root = parse(CIV_VII_ACF).expect("acf parses");
        let state = root.map("AppState").expect("AppState block");
        assert_eq!(state.str("appid"), Some("1295660"));
        assert_eq!(state.str("name"), Some("Sid Meier's Civilization VII"));
        // Case-insensitive lookup of the mixed-case key.
        assert_eq!(state.str("stateflags"), Some("4"));
        assert_eq!(state.str("installdir"), Some("Sid Meier's Civilization VII"));
    }

    #[test]
    fn parses_libraryfolders_paths_with_unescaped_backslashes() {
        let root = parse(LIBRARYFOLDERS).expect("vdf parses");
        let folders = root.map("libraryfolders").expect("libraryfolders block");
        let paths: Vec<&str> = folders
            .entries()
            .iter()
            .filter_map(|(_, v)| match v {
                VdfValue::Map(m) => m.str("path"),
                VdfValue::Str(_) => None,
            })
            .collect();
        assert_eq!(
            paths,
            vec![r"C:\Program Files (x86)\Steam", r"E:\Games\Steam"]
        );
    }

    #[test]
    fn skips_line_comments() {
        let root = parse("\"k\" \"v\" // trailing\n\"k2\" \"v2\"").expect("parses");
        assert_eq!(root.str("k"), Some("v"));
        assert_eq!(root.str("k2"), Some("v2"));
    }

    #[test]
    fn rejects_unterminated_block() {
        assert_eq!(parse("\"AppState\" {"), None);
    }
}
