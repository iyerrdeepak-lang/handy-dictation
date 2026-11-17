//! Spoken dictation commands, in the style of PowerScribe / Dragon.
//!
//! Turns spoken punctuation and layout commands in a transcript into the
//! characters they stand for:
//!
//! | Say                                        | Get            |
//! |--------------------------------------------|----------------|
//! | "period", "full stop"                      | `.`            |
//! | "comma"                                    | `,`            |
//! | "question mark"                            | `?`            |
//! | "exclamation point" / "exclamation mark"   | `!`            |
//! | "colon"                                    | `:`            |
//! | "semicolon"                                | `;`            |
//! | "ellipsis", "dot dot dot"                  | `...`          |
//! | "hyphen"                                   | `-` (no spaces)|
//! | "dash"                                     | ` - `          |
//! | "slash", "forward slash"                   | `/`            |
//! | "open paren" / "close paren"               | `(` / `)`      |
//! | "open quote" / "close quote", "unquote"    | `"` / `"`      |
//! | "new line", "next line"                    | line break     |
//! | "new paragraph", "next paragraph"          | blank line     |
//! | "literal <word>"                           | the word itself|
//!
//! Speech models punctuate on their own, so a dictated command usually
//! arrives wrapped in punctuation the model guessed ("The lungs are clear.
//! Period."). The punctuation the user dictated wins: model punctuation on
//! the command itself is dropped, and so is model punctuation directly before
//! a dictated mark. Sentence capitalization is repaired to match.
//!
//! "period" and "colon" are also ordinary nouns in clinical speech ("the
//! postoperative period", "the sigmoid colon"), so they stay words when the
//! neighbouring words show they are being used as nouns. "literal" forces any
//! command word to be written as a word ("literal comma" → "comma").

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Period,
    Comma,
    Question,
    Exclamation,
    Colon,
    Semicolon,
    Ellipsis,
    Hyphen,
    Dash,
    Slash,
    OpenParen,
    CloseParen,
    OpenQuote,
    CloseQuote,
    NewLine,
    NewParagraph,
    Literal,
}

impl Command {
    /// Marks that attach to the word before them and replace any punctuation
    /// the model put there.
    fn replaces_preceding_punctuation(self) -> bool {
        matches!(
            self,
            Command::Period
                | Command::Comma
                | Command::Question
                | Command::Exclamation
                | Command::Colon
                | Command::Semicolon
                | Command::Ellipsis
                | Command::Hyphen
                | Command::Dash
                | Command::Slash
                | Command::CloseParen
                | Command::CloseQuote
        )
    }
}

/// Spoken phrases, longest first so "question mark" wins over shorter
/// overlaps. Words are compared lowercased with surrounding punctuation
/// removed.
const PHRASES: &[(&[&str], Command)] = &[
    (&["exclamation", "point"], Command::Exclamation),
    (&["exclamation", "mark"], Command::Exclamation),
    (&["question", "mark"], Command::Question),
    (&["full", "stop"], Command::Period),
    (&["semi", "colon"], Command::Semicolon),
    (&["dot", "dot", "dot"], Command::Ellipsis),
    (&["forward", "slash"], Command::Slash),
    (&["open", "parenthesis"], Command::OpenParen),
    (&["open", "parentheses"], Command::OpenParen),
    (&["open", "paren"], Command::OpenParen),
    (&["left", "paren"], Command::OpenParen),
    (&["close", "parenthesis"], Command::CloseParen),
    (&["close", "parentheses"], Command::CloseParen),
    (&["close", "paren"], Command::CloseParen),
    (&["right", "paren"], Command::CloseParen),
    (&["end", "paren"], Command::CloseParen),
    (&["open", "quote"], Command::OpenQuote),
    (&["begin", "quote"], Command::OpenQuote),
    (&["start", "quote"], Command::OpenQuote),
    (&["close", "quote"], Command::CloseQuote),
    (&["end", "quote"], Command::CloseQuote),
    (&["new", "paragraph"], Command::NewParagraph),
    (&["next", "paragraph"], Command::NewParagraph),
    (&["new", "line"], Command::NewLine),
    (&["next", "line"], Command::NewLine),
    (&["newline"], Command::NewLine),
    (&["period"], Command::Period),
    (&["comma"], Command::Comma),
    (&["colon"], Command::Colon),
    (&["semicolon"], Command::Semicolon),
    (&["ellipsis"], Command::Ellipsis),
    (&["hyphen"], Command::Hyphen),
    (&["dash"], Command::Dash),
    (&["slash"], Command::Slash),
    (&["unquote"], Command::CloseQuote),
    (&["literal"], Command::Literal),
];

/// Words that, directly before "period", show it is the noun
/// ("the postoperative period", "a short period").
const NOUN_PERIOD_PRECEDERS: &[&str] = &[
    "the",
    "a",
    "an",
    "this",
    "that",
    "each",
    "every",
    "same",
    "any",
    "time",
    "follow-up",
    "followup",
    "postoperative",
    "post-operative",
    "perioperative",
    "periprocedural",
    "postprocedural",
    "post-procedural",
    "postpartum",
    "peripartum",
    "refractory",
    "latent",
    "incubation",
    "waiting",
    "trial",
    "grace",
    "neonatal",
    "newborn",
    "early",
    "late",
    "short",
    "long",
    "brief",
    "prior",
    "recovery",
    "observation",
    "study",
    "menstrual",
    "washout",
    "window",
    "interim",
    "transition",
    "transitional",
];

/// Words that, directly before "colon", show it is the organ
/// ("the sigmoid colon", "small bowel and colon").
const NOUN_COLON_PRECEDERS: &[&str] = &[
    "the",
    "a",
    "an",
    "sigmoid",
    "transverse",
    "ascending",
    "descending",
    "right",
    "left",
    "proximal",
    "distal",
    "mid",
    "redundant",
    "entire",
    "whole",
    "remaining",
    "decompressed",
    "distended",
    "dilated",
    "visualized",
    "spastic",
    "rectosigmoid",
    "of",
    "and",
    "or",
    "with",
    "without",
    "in",
    "into",
    "to",
    "from",
    "bowel",
    "residual",
    "native",
    "abnormal",
    "normal",
    "unremarkable",
];

/// Words that, directly after "colon", show it is the organ
/// ("colon wall thickening", "colon cancer").
const NOUN_COLON_FOLLOWERS: &[&str] = &[
    "wall",
    "walls",
    "cancer",
    "carcinoma",
    "mass",
    "masses",
    "polyp",
    "polyps",
    "lesion",
    "lesions",
    "is",
    "was",
    "are",
    "were",
    "appears",
    "appeared",
    "demonstrates",
    "demonstrated",
    "shows",
    "showed",
    "contains",
    "contained",
    "diverticula",
    "diverticulum",
    "diverticulosis",
    "diverticulitis",
    "resection",
    "and",
    "or",
    "remains",
    "remained",
];

/// Words that, directly before "new line" / "next line", show it is a phrase
/// about a line (a catheter, a therapy) rather than a layout command
/// ("a new line was placed", "the next line of therapy").
const NOUN_LINE_PRECEDERS: &[&str] = &[
    "a", "an", "the", "this", "that", "his", "her", "their", "its", "another", "one", "any",
    "each", "every", "no",
];

/// Abbreviations whose own period must survive a following dictated mark.
const ABBREVIATIONS: &[&str] = &["dr", "mr", "mrs", "ms", "st", "vs", "etc", "jr", "sr"];

const SENTENCE_MARKS: &[char] = &['.', ',', ';', ':', '!', '?', '…'];

#[derive(Debug)]
enum Piece {
    Word(String),
    Mark {
        command: Command,
        /// The model had ended a sentence where the user dictated this mark,
        /// so the next word's capital is probably the model's, not the user's.
        replaced_sentence_end: bool,
    },
}

/// One whitespace-separated token of the transcript.
struct Token<'a> {
    text: &'a str,
    /// Lowercased word with surrounding punctuation removed.
    key: String,
}

fn word_key(token: &str) -> String {
    token
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn split_hyphenated_commands(text: &str) -> String {
    // "new-line" / "New-paragraph." become two words so they match.
    let mut out = String::with_capacity(text.len());
    for (index, token) in text.split(' ').enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let key = word_key(token);
        if matches!(
            key.as_str(),
            "new-line" | "next-line" | "new-paragraph" | "next-paragraph"
        ) {
            out.push_str(&token.replacen('-', " ", 1));
        } else {
            out.push_str(token);
        }
    }
    out
}

fn match_phrase(tokens: &[Token], start: usize) -> Option<(Command, usize)> {
    PHRASES.iter().find_map(|(words, command)| {
        let end = start + words.len();
        if end > tokens.len() {
            return None;
        }
        tokens[start..end]
            .iter()
            .zip(words.iter())
            .all(|(token, word)| token.key == *word)
            .then_some((*command, words.len()))
    })
}

fn ends_with_sentence_mark(text: &str) -> bool {
    text.ends_with(SENTENCE_MARKS)
}

/// True when a "period", "colon" or "new line" at `index` (spanning `len`
/// words) is being used as ordinary words rather than as a command.
fn is_noun_use(command: Command, tokens: &[Token], index: usize, len: usize) -> bool {
    let previous = index
        .checked_sub(1)
        .map(|i| &tokens[i])
        // A model-inserted sentence break before it means it starts a clause,
        // so the previous word says nothing about it.
        .filter(|token| !ends_with_sentence_mark(token.text));
    let next = tokens.get(index + len);
    let preceded_by = |list: &[&str]| previous.is_some_and(|t| list.contains(&t.key.as_str()));
    match command {
        Command::Period if len == 1 => preceded_by(NOUN_PERIOD_PRECEDERS),
        Command::Colon if len == 1 => {
            preceded_by(NOUN_COLON_PRECEDERS)
                || (!ends_with_sentence_mark(tokens[index].text)
                    && next.is_some_and(|t| NOUN_COLON_FOLLOWERS.contains(&t.key.as_str())))
        }
        Command::NewLine if len == 2 => {
            preceded_by(NOUN_LINE_PRECEDERS)
                || (!ends_with_sentence_mark(tokens[index + 1].text)
                    && next.is_some_and(|t| t.key == "of"))
        }
        _ => false,
    }
}

/// Removes the model's sentence punctuation from the end of a word, keeping
/// closing brackets/quotes and abbreviation periods. Returns whether a
/// sentence-ending mark was removed.
fn strip_model_punctuation(word: &mut String) -> bool {
    let key = word_key(word);
    if ABBREVIATIONS.contains(&key.as_str()) || key.contains('.') {
        return false;
    }
    // Keep acronym-style dotted words ("e.g.", "a.m.") intact.
    let core = word.trim_end_matches(SENTENCE_MARKS);
    if core.contains('.') {
        return false;
    }
    let removed = &word[core.len()..];
    let ended_sentence = removed.contains(['.', '!', '?', '…']);
    let keep = core.len();
    word.truncate(keep);
    ended_sentence
}

fn parse(text: &str) -> Option<Vec<Piece>> {
    let tokens: Vec<Token> = text
        .split(' ')
        .filter(|t| !t.is_empty())
        .map(|t| Token {
            text: t,
            key: word_key(t),
        })
        .collect();

    let mut pieces: Vec<Piece> = Vec::with_capacity(tokens.len());
    let mut matched_any = false;
    let mut index = 0;
    while index < tokens.len() {
        let Some((command, len)) = match_phrase(&tokens, index) else {
            pieces.push(Piece::Word(tokens[index].text.to_string()));
            index += 1;
            continue;
        };

        if command == Command::Literal {
            matched_any = true;
            match tokens.get(index + 1) {
                Some(next) => {
                    pieces.push(Piece::Word(next.text.to_string()));
                    index += 2;
                }
                None => {
                    pieces.push(Piece::Word(tokens[index].text.to_string()));
                    index += 1;
                }
            }
            continue;
        }

        if is_noun_use(command, &tokens, index, len) {
            pieces.extend(
                tokens[index..index + len]
                    .iter()
                    .map(|t| Piece::Word(t.text.to_string())),
            );
            index += len;
            continue;
        }

        matched_any = true;
        let mut replaced_sentence_end = tokens[index + len - 1]
            .text
            .trim_end_matches(['"', ')'])
            .ends_with(['.', '!', '?', '…']);
        if command.replaces_preceding_punctuation() {
            if let Some(Piece::Word(previous)) = pieces.last_mut() {
                replaced_sentence_end |= strip_model_punctuation(previous);
            }
        }
        pieces.push(Piece::Mark {
            command,
            replaced_sentence_end,
        });
        index += len;
    }

    matched_any.then_some(pieces)
}

fn trim_trailing_spaces(out: &mut String) {
    let keep = out.trim_end_matches(' ').len();
    out.truncate(keep);
}

fn needs_space_before(out: &str, no_space_next: bool) -> bool {
    !(out.is_empty() || out.ends_with('\n') || no_space_next)
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    let mut out = String::with_capacity(word.len());
    // Skip leading punctuation such as an opening quote.
    for c in chars.by_ref() {
        if c.is_alphabetic() {
            out.extend(c.to_uppercase());
            out.push_str(chars.as_str());
            return out;
        }
        out.push(c);
    }
    out
}

/// Undoes a capital the model added only because it had started a sentence.
fn uncapitalize_first(word: &str) -> String {
    let key = word_key(word);
    let is_pronoun_i = matches!(key.as_str(), "i" | "i'm" | "i've" | "i'll" | "i'd");
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    let is_acronym = letters.len() > 1 && letters[1].is_uppercase();
    if is_pronoun_i || is_acronym {
        return word.to_string();
    }
    let mut out = String::with_capacity(word.len());
    let mut done = false;
    for c in word.chars() {
        if !done && c.is_alphabetic() {
            out.extend(c.to_lowercase());
            done = true;
        } else {
            out.push(c);
        }
    }
    out
}

fn render(pieces: Vec<Piece>) -> String {
    let mut out = String::new();
    let mut capitalize_next = false;
    let mut lowercase_next = false;
    let mut no_space_next = false;

    for piece in pieces {
        match piece {
            Piece::Word(word) => {
                if needs_space_before(&out, no_space_next) {
                    out.push(' ');
                }
                let word = if capitalize_next {
                    capitalize_first(&word)
                } else if lowercase_next {
                    uncapitalize_first(&word)
                } else {
                    word
                };
                out.push_str(&word);
                capitalize_next = false;
                lowercase_next = false;
                no_space_next = false;
            }
            Piece::Mark {
                command,
                replaced_sentence_end,
            } => {
                lowercase_next = false;
                match command {
                    Command::Period | Command::Question | Command::Exclamation => {
                        trim_trailing_spaces(&mut out);
                        out.push(match command {
                            Command::Period => '.',
                            Command::Question => '?',
                            _ => '!',
                        });
                        capitalize_next = true;
                        no_space_next = false;
                    }
                    Command::Comma | Command::Semicolon => {
                        trim_trailing_spaces(&mut out);
                        out.push(if command == Command::Comma { ',' } else { ';' });
                        lowercase_next = replaced_sentence_end;
                        capitalize_next = false;
                        no_space_next = false;
                    }
                    Command::Colon => {
                        trim_trailing_spaces(&mut out);
                        out.push(':');
                        no_space_next = false;
                    }
                    Command::Ellipsis => {
                        trim_trailing_spaces(&mut out);
                        out.push_str("...");
                        no_space_next = false;
                    }
                    Command::Hyphen | Command::Slash => {
                        trim_trailing_spaces(&mut out);
                        out.push(if command == Command::Hyphen { '-' } else { '/' });
                        lowercase_next = replaced_sentence_end;
                        no_space_next = true;
                    }
                    Command::Dash => {
                        trim_trailing_spaces(&mut out);
                        if !out.is_empty() && !out.ends_with('\n') {
                            out.push(' ');
                        }
                        out.push('-');
                        no_space_next = false;
                    }
                    Command::OpenParen | Command::OpenQuote => {
                        if needs_space_before(&out, no_space_next) {
                            out.push(' ');
                        }
                        out.push(if command == Command::OpenParen {
                            '('
                        } else {
                            '"'
                        });
                        no_space_next = true;
                    }
                    Command::CloseParen | Command::CloseQuote => {
                        trim_trailing_spaces(&mut out);
                        out.push(if command == Command::CloseParen {
                            ')'
                        } else {
                            '"'
                        });
                        no_space_next = false;
                    }
                    Command::NewLine | Command::NewParagraph => {
                        trim_trailing_spaces(&mut out);
                        out.push_str(if command == Command::NewLine {
                            "\n"
                        } else {
                            "\n\n"
                        });
                        capitalize_next = true;
                        no_space_next = false;
                    }
                    Command::Literal => {}
                }
            }
        }
    }

    trim_trailing_spaces(&mut out);
    out
}

/// Applies spoken punctuation and layout commands to a transcript.
///
/// Text without any command is returned unchanged.
pub fn apply_voice_commands(text: &str) -> String {
    let prepared = split_hyphenated_commands(text);
    match parse(&prepared) {
        Some(pieces) => render(pieces),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::apply_voice_commands as vc;

    #[test]
    fn leaves_text_without_commands_untouched() {
        assert_eq!(vc("The lungs are clear."), "The lungs are clear.");
        assert_eq!(vc(""), "");
    }

    #[test]
    fn basic_punctuation() {
        assert_eq!(
            vc("the lungs are clear period no pleural effusion period"),
            "the lungs are clear. No pleural effusion."
        );
        assert_eq!(vc("is it stable question mark"), "is it stable?");
        assert_eq!(vc("wow exclamation point"), "wow!");
        assert_eq!(
            vc("nodule comma measuring 5 mm semicolon stable"),
            "nodule, measuring 5 mm; stable"
        );
    }

    #[test]
    fn model_punctuation_around_commands_is_replaced() {
        assert_eq!(
            vc("The lungs are clear. Period. No effusion. Period."),
            "The lungs are clear. No effusion."
        );
        assert_eq!(
            vc("There is a nodule. Comma, measuring 5 mm."),
            "There is a nodule, measuring 5 mm."
        );
        assert_eq!(
            vc("There is a nodule comma. Measuring 5 mm period."),
            "There is a nodule, measuring 5 mm."
        );
        assert_eq!(vc("Is it stable, question mark?"), "Is it stable?");
    }

    #[test]
    fn new_line_and_new_paragraph() {
        assert_eq!(
            vc("Findings colon new line the liver is normal period new paragraph impression colon no acute findings period"),
            "Findings:\nThe liver is normal.\n\nImpression: no acute findings."
        );
        assert_eq!(
            vc("Normal study. New paragraph. Impression: stable."),
            "Normal study.\n\nImpression: stable."
        );
        assert_eq!(vc("first next line second"), "first\nSecond");
        assert_eq!(vc("new-paragraph the end"), "\n\nThe end");
        assert_eq!(vc("Newline. Done."), "\nDone.");
    }

    #[test]
    fn utterance_that_is_only_a_command() {
        assert_eq!(vc("New paragraph."), "\n\n");
        assert_eq!(vc("New line"), "\n");
        assert_eq!(vc("Period."), ".");
    }

    #[test]
    fn colon_the_organ_stays_a_word() {
        assert_eq!(
            vc("the sigmoid colon is unremarkable period"),
            "the sigmoid colon is unremarkable."
        );
        assert_eq!(
            vc("diverticulosis of the colon period"),
            "diverticulosis of the colon."
        );
        assert_eq!(vc("colon wall thickening"), "colon wall thickening");
        assert_eq!(
            vc("Impression colon the colon is normal"),
            "Impression: the colon is normal"
        );
    }

    #[test]
    fn period_the_noun_stays_a_word() {
        assert_eq!(
            vc("stable over the follow-up period period"),
            "stable over the follow-up period."
        );
        assert_eq!(
            vc("in the postoperative period comma no complications"),
            "in the postoperative period, no complications"
        );
    }

    #[test]
    fn line_the_noun_stays_words() {
        assert_eq!(vc("a new line was placed period"), "a new line was placed.");
        assert_eq!(
            vc("the next line of therapy comma per oncology"),
            "the next line of therapy, per oncology"
        );
        assert_eq!(
            vc("old line removed period new line the catheter is patent"),
            "old line removed.\nThe catheter is patent"
        );
    }

    #[test]
    fn literal_escapes_a_command_word() {
        assert_eq!(
            vc("type the word literal comma here"),
            "type the word comma here"
        );
        assert_eq!(vc("say literal"), "say literal");
    }

    #[test]
    fn brackets_quotes_and_joiners() {
        assert_eq!(
            vc("nodule open paren series 3 image 45 close paren"),
            "nodule (series 3 image 45)"
        );
        assert_eq!(
            vc("he said open quote hello close quote"),
            "he said \"hello\""
        );
        assert_eq!(vc("and slash or"), "and/or");
        assert_eq!(vc("T2 hyphen weighted"), "T2-weighted");
        assert_eq!(vc("first dash second"), "first - second");
        assert_eq!(vc("wait ellipsis okay"), "wait... okay");
    }

    #[test]
    fn keeps_abbreviations_and_acronyms() {
        assert_eq!(vc("see Dr. comma Smith"), "see Dr., Smith");
        assert_eq!(vc("e.g. comma this"), "e.g., this");
        assert_eq!(vc("Nodule. Comma, CT confirms"), "Nodule, CT confirms");
    }
}
