use std::{
    borrow::Cow,
    io::{Read, Write},
    path::Path,
};

use furigana_gen::FuriganaGenerator;

fn main() {
    let content = {
        let in_filename = std::env::args().nth(1).unwrap();
        let mut in_archive =
            zip::ZipArchive::new(std::fs::File::open(in_filename).unwrap()).unwrap();

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

    let out_filename = std::env::args().nth(2).unwrap();
    let mut out_archive = zip::ZipWriter::new(std::fs::File::create(out_filename).unwrap());

    // Write mimetype file first, uncompressed.
    out_archive
        .start_file(
            "mimetype",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    out_archive.write_all(b"application/epub+zip").unwrap();

    let furigen = FuriganaGenerator::new(10, true, true);
    let mut session = furigen.new_session(true);

    // Then write the other files.
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
                Cow::Owned(
                    (text.to_string()
                        + r#"
span.pitch_accent {
    display: inline-block;
    padding-right: 0.15em;
    border-right: solid 0.12rem #c0c0c0;
    border-bottom-right-radius: 0.5em;
}
rt span.pitch_accent {
    padding-right: 0.1em;
}

span.pitch_flat {
    display: inline-block;
    padding-right: 0.15em;
    border-right: solid 0.075rem #c0c0c0;
}
rt span.pitch_flat {
    padding-right: 0.1em;
}
"#)
                    .into_bytes(),
                )
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
}
