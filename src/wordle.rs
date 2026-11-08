//! The Wordle demo: guess the five-letter word in six tries.
//!
//! Each guess is scored like Wordle does it: a letter is green when it is in
//! the right place, yellow when the word has it elsewhere, and gray when it
//! has no more copies left. Repeated letters are counted carefully, so a
//! yellow one is never claimed twice.
//!
//! The answer is picked from a list of common five-letter words, and guesses
//! are checked against a larger list. The pick is seeded from the host's
//! clock, which the app learns on its first tick; a `word` parameter in the
//! address starts a game with that answer instead.

use hypereffect::{Action, App, Handler, Map, Metadata, Outbox, Value, cel, json, ui};
use std::collections::BTreeMap;

const WORD_LEN: usize = 5;
const MAX_GUESSES: usize = 6;
const ALPHABET: usize = 26;

/// The words a game can be about, one per line.
const ANSWERS: &str = include_str!("wordle/answers.txt");
/// Every guess that is accepted, one per line.
const ALLOWED: &str = include_str!("wordle/allowed.txt");

/// How one letter of a guess compares with the answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mark {
    Absent,
    Present,
    Correct,
}

/// One finished guess and its marks.
struct Attempt {
    word: String,
    marks: [Mark; WORD_LEN],
}

/// SplitMix64, seeded from the clock or from a chosen word.
struct Rng(u64);

impl Rng {
    fn draw(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.draw() % n as u64) as usize
    }
}

#[derive(Default)]
pub struct Wordle {
    /// The source of answers; `None` until the first tick.
    rng: Option<Rng>,
    target: String,
    guesses: Vec<Attempt>,
    message: String,
    message_kind: &'static str,
}

impl Wordle {
    fn ready(&self) -> bool {
        self.rng.is_some()
    }

    fn won(&self) -> bool {
        self.guesses.last().is_some_and(|a| a.word == self.target)
    }

    fn over(&self) -> bool {
        self.won() || self.guesses.len() >= MAX_GUESSES
    }

    fn says(&mut self, kind: &'static str, text: impl Into<String>) {
        self.message_kind = kind;
        self.message = text.into();
    }

    /// Starts over with a fresh answer.
    fn new_game(&mut self) {
        if let Some(rng) = self.rng.as_mut() {
            let count = ANSWERS.lines().count();
            self.target = ANSWERS
                .lines()
                .nth(rng.below(count))
                .unwrap_or("crane")
                .to_owned();
        }
        self.guesses.clear();
        self.message.clear();
        self.message_kind = "info";
    }

    /// Accepts `typed` as a guess if it is an allowed word. The input is
    /// cleared only when the guess is taken, so a rejected one can be fixed.
    fn submit(&mut self, out: &mut Outbox, typed: &str) {
        let guess = typed.trim().to_ascii_lowercase();
        if !self.ready() {
            self.says("info", "One moment, the word is being picked.");
        } else if !ALLOWED.lines().any(|w| w == guess) {
            self.says(
                "warning",
                format!("{} is not in the word list.", guess.to_ascii_uppercase()),
            );
        } else if self.guesses.iter().any(|a| a.word == guess) {
            self.says("warning", "You already tried that word.");
        } else {
            let marks = evaluate(&guess, &self.target);
            self.guesses.push(Attempt { word: guess, marks });
            out.signal(json!({"game": {"input": ""}}));
            if self.won() {
                let n = self.guesses.len();
                self.says(
                    "success",
                    format!(
                        "You got it in {n} {}.",
                        if n == 1 { "try" } else { "tries" }
                    ),
                );
            } else if self.guesses.len() >= MAX_GUESSES {
                self.says(
                    "failure",
                    format!(
                        "Out of guesses. The word was {}.",
                        self.target.to_ascii_uppercase()
                    ),
                );
            } else {
                self.message.clear();
            }
        }
        self.show(out);
    }

    /// The tiles of the board, row by row, as signals.
    fn board(&self) -> Map<String, Value> {
        let mut board = Map::new();
        for r in 0..MAX_GUESSES {
            for c in 0..WORD_LEN {
                let (text, variant) = match self.guesses.get(r) {
                    Some(attempt) => {
                        let letter = attempt.word.as_bytes()[c] as char;
                        let variant = match attempt.marks[c] {
                            Mark::Absent => "neutral",
                            Mark::Present => "warning",
                            Mark::Correct => "success",
                        };
                        (letter.to_ascii_uppercase().to_string(), variant)
                    }
                    None => ("·".to_owned(), "neutral"),
                };
                board.insert(
                    format!("r{r}c{c}"),
                    json!({"text": text, "variant": variant}),
                );
            }
        }
        board
    }

    /// Patches everything that can change during a game.
    fn show(&self, out: &mut Outbox) {
        out.signal(json!({
            "game": {
                "ready": self.ready(),
                "over": self.over(),
                "message": self.message,
                "kind": self.message_kind,
            },
            "board": self.board(),
        }));
    }
}

/// Scores `guess` against `answer`, both five lowercase ASCII letters.
fn evaluate(guess: &str, answer: &str) -> [Mark; WORD_LEN] {
    let (guess, answer) = (guess.as_bytes(), answer.as_bytes());
    let mut marks = [Mark::Absent; WORD_LEN];
    let mut pool = [0u8; ALPHABET];
    for i in 0..WORD_LEN {
        if guess[i] == answer[i] {
            marks[i] = Mark::Correct;
        } else {
            pool[(answer[i] - b'a') as usize] += 1;
        }
    }
    for i in 0..WORD_LEN {
        if marks[i] == Mark::Correct {
            continue;
        }
        let slot = &mut pool[(guess[i] - b'a') as usize];
        if *slot > 0 {
            marks[i] = Mark::Present;
            *slot -= 1;
        }
    }
    marks
}

fn seed(word: &str) -> u64 {
    word.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

impl App for Wordle {
    fn hello(&mut self, out: &mut Outbox, params: &BTreeMap<String, String>) {
        if let Some(word) = params
            .get("word")
            .map(|w| w.to_ascii_lowercase())
            .filter(|w| w.len() == WORD_LEN && w.bytes().all(|b| b.is_ascii_lowercase()))
            .filter(|w| ALLOWED.lines().any(|a| a == w))
        {
            self.rng = Some(Rng(seed(&word)));
            self.target = word;
        }
        out.metadata(
            Metadata::new()
                .name("Wordle")
                .description("Guess the five-letter word in six tries."),
        );
        if !self.ready() {
            self.says("info", "Picking a word…");
        }
        out.signal(json!({"game": {"input": ""}}));
        self.show(out);

        out.element(ui::column("main").gap("large"));
        out.element(
            ui::text("title")
                .parent("main")
                .variant("h1")
                .text("Wordle"),
        );
        out.element(
            ui::text("subtitle")
                .parent("main")
                .variant("caption")
                .text("Guess the five-letter word in six tries."),
        );
        out.element(
            ui::card("board_card")
                .parent("main")
                .title("Your guesses")
                .gap("medium"),
        );
        for r in 0..MAX_GUESSES {
            let row = format!("board_{r}");
            out.element(
                ui::row(&row)
                    .parent("board_card")
                    .gap("small")
                    .align("center")
                    .justify("center"),
            );
            for c in 0..WORD_LEN {
                let path = format!("board.r{r}c{c}");
                out.element(
                    ui::badge(format!("tile_{r}_{c}"))
                        .parent(&row)
                        .variant(cel(format!("s.{path}.variant")))
                        .text(cel(format!("s.{path}.text"))),
                );
            }
        }
        out.element(
            ui::row("entry")
                .parent("board_card")
                .gap("medium")
                .align("end")
                .wrap(true),
        );
        out.element(
            ui::input("guess")
                .parent("entry")
                .label("Your guess")
                .placeholder("Five letters")
                .bind("game.input")
                .disabled(cel("s.game.over || !s.game.ready"))
                .on_submit(Handler::action("submit")),
        );
        out.element(
            ui::button("submit")
                .parent("entry")
                .label("Guess")
                .variant("primary")
                .disabled(cel("s.game.over || !s.game.ready"))
                .on_press(Handler::action("submit")),
        );
        out.element(
            ui::notice("message")
                .parent("main")
                .variant(cel("s.game.kind"))
                .visible(cel("s.game.message != ''"))
                .text(cel("s.game.message")),
        );
        out.element(
            ui::row("controls")
                .parent("main")
                .gap("medium")
                .justify("center"),
        );
        out.element(
            ui::button("new")
                .parent("controls")
                .label("New game")
                .variant("primary")
                .on_press(Handler::action("new")),
        );
        out.element(ui::text("help").parent("main").variant("caption").text(
            "Green: right letter, right place. Yellow: right letter, \
                     wrong place. Gray: not in the word.",
        ));
    }

    fn action(&mut self, out: &mut Outbox, action: &Action) {
        let typed = action
            .signal("game.input")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match action.name.as_str() {
            "submit" => {
                if self.over() {
                    return;
                }
                self.submit(out, typed);
            }
            "new" => {
                if !self.ready() {
                    self.says("info", "One moment, the word is being picked.");
                    self.show(out);
                    return;
                }
                self.new_game();
                out.signal(json!({"game": {"input": ""}}));
                self.show(out);
            }
            _ => {}
        }
    }

    fn tick(&mut self, out: &mut Outbox, now_ms: f64) {
        if self.ready() {
            return;
        }
        self.rng = Some(Rng(now_ms.to_bits() ^ 0x9e37_79b9_7f4a_7c15));
        if self.target.is_empty() {
            self.new_game();
        }
        self.show(out);
    }

    /// One tick to learn the clock, then no more.
    fn tick_interval(&self) -> Option<u32> {
        (!self.ready()).then_some(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypereffect::server::Loopback;

    fn game(word: &str) -> Loopback<Wordle> {
        Loopback::with_params(&BTreeMap::from([("word".to_owned(), word.to_owned())]))
    }

    #[test]
    fn marks_count_repeated_letters_once() {
        assert_eq!(evaluate("crane", "crane"), [Mark::Correct; WORD_LEN]);
        assert_eq!(
            evaluate("allee", "eagle"),
            [
                Mark::Present,
                Mark::Present,
                Mark::Absent,
                Mark::Present,
                Mark::Correct
            ]
        );
        // The first s is present, the second has no copy left, and both a's
        // find their own answer.
        assert_eq!(
            evaluate("sassy", "assay"),
            [
                Mark::Present,
                Mark::Present,
                Mark::Correct,
                Mark::Absent,
                Mark::Correct
            ]
        );
    }

    #[test]
    fn a_word_parameter_starts_that_game() {
        let mut l = game("crane");
        assert_eq!(l.client.metadata().name.as_deref(), Some("Wordle"));
        assert_eq!(l.client.attr("guess", "disabled"), Some(json!(false)));
        l.write("game.input", json!("crane"));
        assert!(l.trigger("guess", "onSubmit"));
        assert_eq!(l.client.attr("message", "variant"), Some(json!("success")));
        assert_eq!(
            l.client.attr("message", "text"),
            Some(json!("You got it in 1 try."))
        );
        assert_eq!(l.client.attr("tile_0_0", "variant"), Some(json!("success")));
        assert!(
            l.client
                .attr("guess", "disabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn bad_guesses_are_explained() {
        let mut l = game("crane");
        l.write("game.input", json!("abc"));
        assert!(l.trigger("guess", "onSubmit"));
        assert_eq!(l.client.attr("message", "variant"), Some(json!("warning")));
        assert!(
            l.client
                .attr("message", "text")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("not in the word list")
        );
        l.write("game.input", json!("zzzzz"));
        assert!(l.trigger("guess", "onSubmit"));
        assert_eq!(
            l.client.attr("message", "text"),
            Some(json!("ZZZZZ is not in the word list."))
        );
        // The board did not grow on either attempt, and the rejected word
        // stays in the input to be fixed.
        assert!(l.server.app().unwrap().guesses.is_empty());
        assert_eq!(l.client.value("game.input"), Some(json!("zzzzz")).as_ref());
        l.write("game.input", json!("aahed"));
        assert!(l.trigger("guess", "onSubmit"));
        l.write("game.input", json!("aahed"));
        assert!(l.trigger("guess", "onSubmit"));
        assert_eq!(
            l.client.attr("message", "text"),
            Some(json!("You already tried that word."))
        );
        assert_eq!(l.server.app().unwrap().guesses.len(), 1);
    }

    #[test]
    fn losing_ends_the_game() {
        let mut l = game("crane");
        let words: Vec<&str> = ALLOWED
            .lines()
            .filter(|w| *w != "crane")
            .take(MAX_GUESSES)
            .collect();
        for word in words {
            l.write("game.input", json!(word));
            assert!(l.trigger("guess", "onSubmit"));
        }
        assert!(l.server.app().unwrap().over());
        assert!(
            l.client
                .attr("message", "text")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("The word was CRANE.")
        );
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn a_new_game_clears_the_board() {
        let mut l = game("crane");
        l.write("game.input", json!("crane"));
        assert!(l.trigger("guess", "onSubmit"));
        assert!(l.trigger("new", "onPress"));
        assert_eq!(l.client.attr("message", "visible"), Some(json!(false)));
        assert_eq!(l.client.attr("tile_0_0", "text"), Some(json!("·")));
        assert_eq!(l.client.value("game.input"), Some(json!("")).as_ref());
        assert!(l.client.attr("guess", "disabled") == Some(json!(false)));
        assert_ne!(l.server.app().unwrap().target, "crane");
    }

    #[test]
    fn the_first_tick_picks_a_word() {
        let mut l = Loopback::<Wordle>::new();
        assert_eq!(l.server.tick_interval(), Some(1));
        assert!(
            l.client
                .attr("guess", "disabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
        l.tick(1_700_000_000_000.0);
        assert_eq!(l.server.tick_interval(), None);
        assert!(
            !l.client
                .attr("guess", "disabled")
                .unwrap()
                .as_bool()
                .unwrap()
        );
        assert_eq!(l.client.attr("message", "visible"), Some(json!(false)));
        let word = &l.server.app().unwrap().target;
        assert_eq!(word.len(), WORD_LEN);
        assert!(ANSWERS.lines().any(|w| w == word));
        assert!(l.client.take_diagnostics().is_empty());
    }
}
