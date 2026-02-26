use rbook::Epub;
use rbook::epub::EpubOpenOptions;
use std::io::Cursor;

pub const EPUB3_DIR: &str = "tests/ebooks/example_epub";
const EPUB3_RELAXED: &str = "tests/ebooks/epub3_relaxed";
const EPUB2_DIR: &str = "tests/ebooks/epub2";

const EPUB3_FILE_BYTES: &[u8] = include_bytes!("../../tests/ebooks/example.epub");

pub enum TestEpub {
    /// Unzipped Epub `2` + `3` directory
    ///
    /// Mapped to: [`EPUB3_DIR`]
    Epub3Dir,
    /// Zipped Epub `2` + `3` File
    ///
    /// Mapped to: `tests/ebooks/example.epub`
    Epub3File,
    /// Zipped malformed Epub `2` + `3` File
    ///
    /// Intended to for relaxed parsing (`strict` mode disabled).
    ///
    /// Mapped to: [`EPUB3_RELAXED`]
    Epub3Relaxed,
    /// Unzipped Epub `2` directory
    ///
    /// Mapped to: [`EPUB2_DIR`]
    Epub2Dir,
}

impl TestEpub {
    pub fn open_strict(self) -> Epub {
        self.build(|b| b.strict(true))
    }

    pub fn build(self, builder: impl Fn(&mut EpubOpenOptions) -> &mut EpubOpenOptions) -> Epub {
        let mut options = EpubOpenOptions::default();
        builder(&mut options);

        // File bytes
        if matches!(self, Self::Epub3File) {
            let cursor = Cursor::new(EPUB3_FILE_BYTES);
            options.read(cursor).unwrap()
        }
        // Directory
        else {
            options
                .open(match self {
                    Self::Epub3Dir => EPUB3_DIR,
                    Self::Epub3Relaxed => EPUB3_RELAXED,
                    Self::Epub2Dir => EPUB2_DIR,
                    _ => panic!("Unexpected test file provided"),
                })
                .unwrap()
        }
    }
}

#[cfg(feature = "write")]
pub fn round_trip_epub(epub: &Epub) -> Epub {
    let bytes = epub
        .write()
        .compression(0)
        .to_vec()
        .expect("All content should be written");

    Epub::read(Cursor::new(bytes)).expect("EPUB should be valid")
}
