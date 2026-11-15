//! How the user's clock is written: the short time pattern and the words for morning and
//! afternoon. The shell reads them from Windows' settings and hands them in.
use chrono::{NaiveTime, Timelike};

/// The way the user writes a time of day. The pattern is in the syntax of .NET's custom time
/// patterns, as 1.x read it from .NET; the shell turns Windows' time picture into it (a quote
/// inside quoted text is written `''` in Windows' picture), and writes the time separator into
/// the pattern itself, since `:` here is copied as it is, where .NET put the culture's time
/// separator. The pattern uses these letters: `h` and `hh` for the hour on a 12-hour clock, `H`
/// and `HH` for the hour on a 24-hour clock, `m` and `mm` for the minute, `s` and `ss` for the
/// second, `t` for the first letter of the word for morning or afternoon and `tt` for the whole
/// word. A longer run of one letter counts as two, as it does in .NET. Text in single or double
/// quotes is copied as it is, a backslash copies the one character after it, and a `%` before a
/// letter makes that letter a pattern of its own. Every other character is copied as it is, `:`
/// and `.` included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockFormat {
    pub pattern: String,
    /// The word for the hours before noon.
    pub am: String,
    /// The word for noon and after.
    pub pm: String,
}

impl ClockFormat {
    /// A time of day written as the pattern says, for example `2:20 PM` or `14:20`.
    pub fn format(&self, time: NaiveTime) -> String {
        let pattern: Vec<char> = self.pattern.chars().collect();
        let mut text = String::new();
        let mut at = 0;
        while let Some(&letter) = pattern.get(at) {
            match letter {
                'h' | 'H' | 'm' | 's' | 't' => {
                    let run = pattern[at..]
                        .iter()
                        .take_while(|&&next| next == letter)
                        .count();
                    self.write_field(&mut text, letter, run, time);
                    at += run;
                }
                '\'' | '"' => at = copy_quoted(&pattern, at, &mut text),
                '\\' => match pattern.get(at + 1) {
                    Some(&escaped) => {
                        text.push(escaped);
                        at += 2;
                    }
                    None => {
                        text.push(letter);
                        at += 1;
                    }
                },
                '%' => match pattern.get(at + 1) {
                    Some(&single) if single != '%' => {
                        match single {
                            'h' | 'H' | 'm' | 's' | 't' => {
                                self.write_field(&mut text, single, 1, time)
                            }
                            other => text.push(other),
                        }
                        at += 2;
                    }
                    _ => {
                        text.push(letter);
                        at += 1;
                    }
                },
                other => {
                    text.push(other);
                    at += 1;
                }
            }
        }
        text
    }

    /// One letter of the pattern, repeated `run` times in a row.
    fn write_field(&self, text: &mut String, letter: char, run: usize, time: NaiveTime) {
        let digits = run.min(2);
        match letter {
            'h' => {
                let hour = time.hour() % 12;
                write_number(text, if hour == 0 { 12 } else { hour }, digits);
            }
            'H' => write_number(text, time.hour(), digits),
            'm' => write_number(text, time.minute(), digits),
            's' => write_number(text, time.second(), digits),
            _ => {
                let word = if time.hour() < 12 { &self.am } else { &self.pm };
                if run == 1 {
                    text.extend(word.chars().next());
                } else {
                    text.push_str(word);
                }
            }
        }
    }
}

/// A number, with a leading zero when two digits are asked for.
fn write_number(text: &mut String, number: u32, digits: usize) {
    text.push_str(&format!("{number:0digits$}"));
}

/// Copies the text between the quotes that open at `start`, and returns where the pattern goes
/// on. A backslash inside the quotes copies the character after it. Quotes that never close
/// run to the end of the pattern.
fn copy_quoted(pattern: &[char], start: usize, text: &mut String) -> usize {
    let quote = pattern[start];
    let mut at = start + 1;
    while let Some(&letter) = pattern.get(at) {
        at += 1;
        if letter == quote {
            break;
        }
        if letter == '\\' {
            if let Some(&escaped) = pattern.get(at) {
                text.push(escaped);
                at += 1;
            }
        } else {
            text.push(letter);
        }
    }
    at
}
