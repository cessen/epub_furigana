use std::{
    borrow::Cow,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use furigana_gen::FuriganaGenerator;

const CSS: &str = r#"
ruby.pitch_accent > rt {
    color: #c0c0c0;
}
ruby.pitch_flat > rt {
    color: #c0c0c0;
}
"#;

#[derive(Clone, Debug)]
struct Args {
    pitch_accent: bool,
    furigana_exclude: Option<usize>,
    known_words: Option<String>,
    learn_mode: bool,
    word_stats: bool,
    input_filepath: String,
    output_filepath: String,
}

impl Args {
    fn parse() -> Args {
        use bpaf::{construct, long, positional, Parser};

        let pitch_accent = long("pitch-accent")
            .short('p')
            .help("When adding furigana to a word, include a pitch accent marker when the accent is unambiguous. A curled marker indicates the accented mora, a flat marker indicates flat pitch (heiban).")
            .switch();
        let furigana_exclude = long("furigana-exclude")
            .short('x')
            .help("Don't add furigana to words made up of the first N most common kanji.")
            .argument::<usize>("N")
            .optional();
        let known_words = long("known-words")
            .short('k')
            .help("Don't add furigana to words in this text file.")
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
        let input_filepath =
            positional::<String>("IN_EPUB_FILE").help("Path to the input epub file.");
        let output_filepath =
            positional::<String>("OUT_EPUB_FILE").help("Path to write the processed epub file to.");

        construct!(Args {
            pitch_accent,
            furigana_exclude,
            known_words,
            learn_mode,
            word_stats,
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

        return true;
    }
}

fn main() {
    let args = Args::parse();
    if !args.validate() {
        return;
    }

    // Load known word list if given.
    let known_words_text = if let Some(known_words_path) = args.known_words {
        // TODO: error on file not accessible, etc.
        std::fs::read_to_string(known_words_path).unwrap_or_else(|_| "".into())
    } else {
        "".into()
    };
    let known_words: Vec<&str> = known_words_text.split(char::is_whitespace).collect();

    // Load input file.
    let content = {
        let mut in_archive =
            zip::ZipArchive::new(std::fs::File::open(&args.input_filepath).unwrap()).unwrap();

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
    let mut out_archive =
        zip::ZipWriter::new(std::fs::File::create(&args.output_filepath).unwrap());

    // Prepare furigana generator.
    let furigen = FuriganaGenerator::new(
        args.furigana_exclude.unwrap_or(0),
        &known_words,
        true,
        if args.pitch_accent {
            Some("＊".into())
        } else {
            None
        },
        if args.pitch_accent {
            Some("口".into())
        } else {
            None
        },
    );
    let mut session = furigen.new_session(args.learn_mode);

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
        let filename: String = format!("{}.word_stats.txt", args.output_filepath);

        let (total_words, stats) = session.word_stats();

        let mut f = File::create(&filename).unwrap();
        write!(&mut f, "Text length in words: {}\n\n", total_words).unwrap();
        for (word, max_distance, times_seen) in stats.iter() {
            write!(
                &mut f,
                "{}        distance {} | seen {}\n",
                word, max_distance, times_seen
            )
            .unwrap();
        }
    }
}
