# EPUB Furigana

Adds generated furigana to Japanese .epub ebooks.

Since the furigana is generated, it is of course not perfect.  Its most frequent mistakes are for single-kanji words, which tend to be ambiguous. But otherwise it's generally pretty good.  Always take the furigana with a little grain of salt, however.

In addition to simple furigana generation, EPUB Furigana also has the following optional features:

- A "learn mode" that generates furigana in a spaced-repetition manner, where words that appear frequently in the book slowly show their furigana less and less.
- Adding pitch accent markings to furigana-ized words. A `＊` mark indicates the accented character, and a `口` mark on the last character indicates flat (heiban) pitch.  (The use of a `口` mark rather than no mark to indicate flat pitch is to distinguish it from no indication of pitch accent at all, such as when the pitch accent is unknown, ambiguous, or when words are conjugated, which isn't handled.)


## Building

Ensure that you have the standard [Rust](https://www.rust-lang.org) toolchain
installed.  Then from the repository root simply run:

```
cargo build --release
```


## Usage

```bash
./epub_furigana <input_file> <output_file>
```

For example, to add furigana to 魔女の宅急便.epub,
simply run:

```bash
./epub_furigana 魔女の宅急便.epub 魔女の宅急便_furigana.epub
```

This will create a new file 魔女の宅急便_furigana.epub with generated furigana.

If you would like to generate furigana with the spaced-repetition "learn mode", simply pass the `-l` parameter:

```bash
./epub_furigana -l 魔女の宅急便.epub 魔女の宅急便_furigana.epub
```

And if you want pitch-accent markings, pass the `-p` parameter:

```bash
./epub_furigana -p 魔女の宅急便.epub 魔女の宅急便_furigana.epub
```

These can, of course, also be combined:

```bash
./epub_furigana -lp 魔女の宅急便.epub 魔女の宅急便_furigana.epub
```

There are additional features as well.  Please see the command line help
(`./epub_furigana --help`) for more details.


## License

EPUB Furigana is licensed under the GPLv3 ([LICENSE.md](LICENSE.md) or https://opensource.org/license/gpl-3-0).


## Contributing

Contributions are absolutely welcome!  If you want to make larger changes,
please first open an issue to discuss it to avoid doing a lot of work that may
get rejected.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in EPUB Furigana by you will be licensed as above, without any
additional terms or conditions.
