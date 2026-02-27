use clap::Args;
use rbook::Epub;
use rbook::ebook::errors::EbookResult;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct DebugCommand {
    /// An EPUB file or directory containing the contents of an unzipped EPUB
    pub ebook_path: PathBuf,

    /// Display all metadata
    #[arg(long)]
    metadata: bool,

    /// Display the manifest
    #[arg(long)]
    manifest: bool,

    /// Display the spine
    #[arg(long)]
    spine: bool,

    /// Display the ToC
    #[arg(long)]
    toc: bool,
}

impl DebugCommand {
    pub fn debug(&self) -> EbookResult<()> {
        // Configure epub open options
        let mut options = Epub::options();
        // If any boolean arguments are set, parse the specific selected component
        if self.has_selected_components() {
            options
                .skip_metadata(!self.metadata)
                .skip_manifest(!self.manifest)
                .skip_spine(!self.spine)
                .skip_toc(!self.toc);
        }

        let epub = options.open(&self.ebook_path)?;
        self.show_debug(&epub);

        Ok(())
    }

    pub fn has_selected_components(&self) -> bool {
        self.metadata || self.manifest || self.spine || self.toc
    }

    pub fn show_debug(&self, epub: &Epub) {
        if !self.has_selected_components() {
            println!("{epub:#?}");
            return;
        }

        let debug_structs: &[(bool, &dyn std::fmt::Debug)] = &[
            (self.metadata, &epub.metadata()),
            (self.manifest, &epub.manifest()),
            (self.spine, &epub.spine()),
            (self.toc, &epub.toc()),
        ];

        for (is_print, debug_struct) in debug_structs {
            if *is_print {
                println!("{debug_struct:#?}");
            }
        }
    }
}
