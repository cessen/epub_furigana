use std::{
    borrow::Cow,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use furigana_gen::FuriganaGenerator;

const CSS: &str = r#"
ruby.pitch_accent > rt {
    font-size: 0.25rem;
    color: #808080;
}
ruby.pitch_flat > rt {
    font-size: 0.25rem;
    color: #808080;
}
"#;

/// List of basic grammar-esk or exclamation words, that shouldn't
/// be included in the word stats printed at the end.
const BASIC_WORDS: &[&str] = &[
    "ない",
    "って",
    "です",
    "なっ",
    "ます",
    "する",
    "ある",
    "たら",
    "てる",
    "なる",
    "さん",
    "よう",
    "いる",
    "でも",
    "じゃ",
    "えっ",
    "くれ",
    "こと",
    "とか",
    "たい",
    "だっ",
    "あっ",
    "なぁ",
    "れる",
    "だって",
    "いく",
    "という",
    "くる",
    "なく",
    "ねぇ",
    "やっ",
    "おい",
    "いっ",
    "なり",
    "っと",
    "おく",
    "られ",
];

#[derive(Clone, Debug)]
struct Args {
    pitch_accent: bool,
    known_kanji: Option<String>,
    known_words: Option<String>,
    learn_mode: bool,
    word_stats: bool,
    input_sentence: Option<String>,
    input_filepath: Option<String>,
    output_filepath: Option<String>,
}

impl Args {
    fn parse() -> Args {
        use bpaf::{construct, long, positional, Parser};

        let pitch_accent = long("pitch-accent")
            .short('p')
            .help("When adding furigana to a word, include a pitch accent marker when the accent is unambiguous. A curled marker indicates the accented mora, a flat marker indicates flat pitch (heiban).")
            .switch();
        let known_words = long("known-words")
            .short('w')
            .help("Don't add furigana to words in this text file.")
            .argument::<String>("FILE")
            .optional();
        let known_kanji = long("known-kanji")
            .short('k')
            .help("Don't add furigana to words whose kanji are all in this text file.")
            .argument::<String>("FILE")
            .optional();
        let learn_mode = long("learn-mode")
            .short('l')
            .help("Put furigana on words in a spaced-repitition style, so words that show up frequenly lose their furigana as the book goes on.")
            .switch();
        let word_stats = long("word-stats")
            .short('s')
            .help("When using learning mode, outputs a word stats file showing all the words parsed along with some statistics about them.")
            .switch();
        let input_sentence = long("input_sentence")
            .short('i')
            .argument::<String>("SENTENCE")
            .optional();
        let input_filepath = positional::<String>("IN_EPUB_FILE")
            .help("Path to the input epub file.")
            .optional();
        let output_filepath = positional::<String>("OUT_EPUB_FILE")
            .help("Path to write the processed epub file to.")
            .optional();

        construct!(Args {
            pitch_accent,
            known_kanji,
            known_words,
            learn_mode,
            word_stats,
            input_sentence,
            input_filepath,
            output_filepath
        })
        .to_options()
        .run()
    }

    /// Returns true if all is good, false if there's a problem.
    ///
    /// Prints its own error messages if there's a problem.
    fn validate(&self) -> bool {
        if self.word_stats && !self.learn_mode {
            println!("Error: outputting word stats requires learn mode to be enabled.");
            return false;
        }

        if self.input_sentence.is_some() {
            if self.input_filepath.is_some() || self.output_filepath.is_some() {
                println!("Error: cannot specify both an input sentence and an input/output file.");
                return false;
            }
            return true;
        }

        if self.input_filepath.is_none() || self.output_filepath.is_none() {
            println!("Error: both an input and output file are required (unless you use -i).");
            return false;
        }

        return true;
    }
}

fn main() {
    let args = Args::parse();
    if !args.validate() {
        return;
    }

    // Load known kanji list if given.
    let known_kanji_text = if let Some(known_kanji_path) = args.known_kanji {
        // TODO: error on file not accessible, etc.
        std::fs::read_to_string(known_kanji_path).unwrap_or_else(|_| "".into())
    } else {
        "".into()
    };
    let known_kanji: Vec<char> = known_kanji_text.chars().collect();

    // Load known word list if given.
    let known_words_text = if let Some(known_words_path) = args.known_words {
        // TODO: error on file not accessible, etc.
        std::fs::read_to_string(known_words_path).unwrap_or_else(|_| "".into())
    } else {
        "".into()
    };
    let known_words: Vec<&str> = known_words_text.split(char::is_whitespace).collect();

    // Prepare furigana generator.
    let furigen = FuriganaGenerator::new(
        &known_kanji,
        &known_words,
        true,
        if args.pitch_accent {
            Some("＊".into())
        } else {
            None
        },
        if args.pitch_accent {
            Some("━".into())
        } else {
            None
        },
    );
    let mut session = furigen.new_session(args.learn_mode);

    // We're doing the input sentence, not an epub.
    if let Some(sentence) = args.input_sentence {
        println!("{}", session.add_html_furigana(&sentence));
        return;
    }

    // We're doing an epub.
    let input_filepath = args.input_filepath.unwrap();
    let output_filepath = args.output_filepath.unwrap();

    // Load input file.
    let content = {
        let mut in_archive =
            zip::ZipArchive::new(std::fs::File::open(&input_filepath).unwrap()).unwrap();

        let mut content = Vec::new();
        for i in 0..in_archive.len() {
            let mut file = in_archive.by_index(i).unwrap();

            if !file.is_file() {
                continue;
            }

            let path = match file.enclosed_name() {
                Some(p) => p,
                None => continue,
            };

            if i == 0 {
                assert_eq!(path, Path::new("mimetype"), "Not a valid epub file.");
                continue;
            }

            let data = {
                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();
                data
            };

            content.push((path, data));
        }

        content
    };

    // Open output file.
    let mut out_archive = zip::ZipWriter::new(std::fs::File::create(&output_filepath).unwrap());

    // Write mimetype file first, uncompressed.
    out_archive
        .start_file(
            "mimetype",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    out_archive.write_all(b"application/epub+zip").unwrap();

    // Then write the other files, processing to add furigana as appropriate.
    for (path, data) in &content {
        println!("Writing {}", path.display());

        let path_str = path.to_string_lossy();

        let data = if (path_str.ends_with(".html") || path_str.ends_with(".xhtml"))
            && !path_str.contains("nav")
        {
            if let Ok(page) = std::str::from_utf8(data) {
                Cow::Owned(session.add_html_furigana(&page).into_bytes())
            } else {
                Cow::Borrowed(data)
            }
        } else if path_str.ends_with(".css") {
            if let Ok(text) = std::str::from_utf8(data) {
                Cow::Owned((text.to_string() + CSS).into_bytes())
            } else {
                Cow::Borrowed(data)
            }
        } else {
            Cow::Borrowed(data)
        };

        out_archive
            .start_file_from_path(
                path,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        out_archive.write_all(&data).unwrap();
    }

    out_archive.finish().unwrap();

    // Save word stats to a text file.
    if args.word_stats {
        // Output filename.
        let filename: String = format!("{}.word_stats.txt", &output_filepath);

        let (total_words, stats) = session.word_stats();

        let mut f = File::create(&filename).unwrap();
        write!(&mut f, "Text length in words: {}\n\n", total_words).unwrap();
        let mut omitted_words = 0;
        let mut omitted_seen = 0;
        for (word, max_distance, times_seen) in stats.iter() {
            let c = word.chars().next().unwrap();
            if (word.chars().count() == 1 && ((c as u32) < 0x4e00 || (c as u32) > 0x9faf))
                || BASIC_WORDS.contains(&&word[..])
            {
                // We skip "words" that are single-character and aren't
                // kanji, or are part of a list of basic grammar-esk words.
                // This gets rid of punctuation, single-character hiragana,
                // etc.
                omitted_words += 1;
                omitted_seen += times_seen;
                continue;
            }
            write!(
                &mut f,
                "{}        distance {} | seen {}\n",
                word, max_distance, times_seen
            )
            .unwrap();
        }
        write!(
            &mut f,
            "\nOmitted (punctuation, single hiragana, etc.):   words {} | seen {}\n",
            omitted_words, omitted_seen
        )
        .unwrap();
    }
}
