use crate::ebook::errors::EbookError;
use crate::ebook::metadata::datetime::DateTime;
use crate::writer::WriterResult;
use std::io::Write;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime as ZipDateTime};

pub(crate) trait ZipFileOptionsExt {
    fn zip_compression_level(self, level: u8) -> Self;

    fn zip_last_modified_date(self, date: Option<DateTime>) -> Self;
}

impl ZipFileOptionsExt for SimpleFileOptions {
    fn zip_compression_level(self, level: u8) -> Self {
        if level == 0 {
            self.compression_method(CompressionMethod::Stored)
        } else {
            self.compression_level(Some(level as i64))
        }
    }

    fn zip_last_modified_date(self, datetime: Option<DateTime>) -> Self {
        let datetime = datetime.and_then(|datetime| {
            let date = datetime.date();
            let time = datetime.time();

            ZipDateTime::from_date_and_time(
                date.year().clamp(0, 9999) as u16,
                date.month(),
                date.day(),
                time.hour(),
                time.minute(),
                time.second(),
            )
            .ok()
        });

        self.last_modified_time(datetime.unwrap_or_default())
    }
}

pub(crate) struct ZipWriter<W: Write> {
    inner: zip::ZipWriter<zip::write::StreamWriter<W>>,
    options: SimpleFileOptions,
}

impl<W: Write> ZipWriter<W> {
    pub(crate) fn new(writer: W, options: SimpleFileOptions) -> Self {
        Self {
            inner: zip::ZipWriter::new_stream(writer),
            options,
        }
    }

    fn start_zip_file_entry(&mut self, name: &str, options: SimpleFileOptions) -> WriterResult<()> {
        self.inner
            // Strip leading '/' to avoid absolute paths in the archive.
            // `zip::ZipWriter` does not need the root prefix to specify the zip container root.
            .start_file(name.trim_start_matches('/'), options)
            .map_err(from_zip_error)
    }

    pub(crate) fn start_uncompressed_file(&mut self, name: &str) -> WriterResult<()> {
        self.start_zip_file_entry(
            name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
    }

    pub(crate) fn start_file(&mut self, name: &str) -> WriterResult<()> {
        self.start_zip_file_entry(name, self.options)
    }

    pub(crate) fn finish(self) -> WriterResult<W> {
        self.inner
            .finish()
            .map_err(from_zip_error)
            .map(|stream_writer| stream_writer.into_inner())
    }
}

impl<W: Write> Write for ZipWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn from_zip_error(error: zip::result::ZipError) -> EbookError {
    EbookError::Io(match error {
        zip::result::ZipError::Io(error) => error,
        error => std::io::Error::other(error),
    })
}
