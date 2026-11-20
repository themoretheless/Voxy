//! Zero-allocation streaming and multi-threaded canonical JSON CRC-32C computation.
use std::io::{self, Write};

use super::{DocumentError, SceneDocument, SceneObject};

const PARALLEL_THRESHOLD: usize = 256;

struct BufferedCrcWriter {
    buf: [u8; 4096],
    len: usize,
    crc: u32,
}

impl BufferedCrcWriter {
    fn new() -> Self {
        Self {
            buf: [0; 4096],
            len: 0,
            crc: 0,
        }
    }

    fn finish(mut self) -> u32 {
        if self.len > 0 {
            self.crc = voxy_core::crc32c_append(self.crc, &self.buf[..self.len]);
            self.len = 0;
        }
        self.crc
    }
}

impl Write for BufferedCrcWriter {
    fn write(&mut self, mut input: &[u8]) -> io::Result<usize> {
        let total = input.len();
        while !input.is_empty() {
            let space = self.buf.len() - self.len;
            if space == 0 {
                self.crc = voxy_core::crc32c_append(self.crc, &self.buf);
                self.len = 0;
                continue;
            }
            let take = space.min(input.len());
            self.buf[self.len..self.len + take].copy_from_slice(&input[..take]);
            self.len += take;
            input = &input[take..];
        }
        Ok(total)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Streams bytes while expanding every literal `0x0A` (`\n`) into `\n    ` so an independently
/// pretty-printed `SceneObject` matches its indented representation inside `SceneDocument.objects`.
struct IndentedCrcWriter {
    buf: [u8; 4096],
    len: usize,
    crc: u32,
    total_bytes: usize,
}

impl IndentedCrcWriter {
    fn new() -> Self {
        Self {
            buf: [0; 4096],
            len: 0,
            crc: 0,
            total_bytes: 0,
        }
    }

    #[inline]
    fn push_raw(&mut self, bytes: &[u8]) {
        self.total_bytes += bytes.len();
        let mut src = bytes;
        while !src.is_empty() {
            let space = self.buf.len() - self.len;
            if space == 0 {
                self.crc = voxy_core::crc32c_append(self.crc, &self.buf);
                self.len = 0;
                continue;
            }
            let take = space.min(src.len());
            self.buf[self.len..self.len + take].copy_from_slice(&src[..take]);
            self.len += take;
            src = &src[take..];
        }
    }

    fn finish(mut self) -> (u32, usize) {
        if self.len > 0 {
            self.crc = voxy_core::crc32c_append(self.crc, &self.buf[..self.len]);
            self.len = 0;
        }
        (self.crc, self.total_bytes)
    }
}

impl Write for IndentedCrcWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let mut start = 0;
        for (i, &byte) in input.iter().enumerate() {
            if byte == b'\n' {
                if i > start {
                    self.push_raw(&input[start..i]);
                }
                self.push_raw(b"\n    ");
                start = i + 1;
            }
        }
        if start < input.len() {
            self.push_raw(&input[start..]);
        }
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash_object_slice(
    objects: &[SceneObject],
    global_start: usize,
) -> Result<(u32, usize), DocumentError> {
    let mut writer = IndentedCrcWriter::new();
    for (offset, object) in objects.iter().enumerate() {
        object.transform().matrix()?;
        object.checked_precise_translation()?;
        if global_start + offset > 0 {
            writer.push_raw(b",\n");
        }
        writer.push_raw(b"    ");
        serde_json::to_writer_pretty(&mut writer, object)?;
    }
    Ok(writer.finish())
}

pub(super) fn compute(document: &SceneDocument) -> Result<u32, DocumentError> {
    if document.objects.len() < PARALLEL_THRESHOLD {
        for object in &document.objects {
            object.transform().matrix()?;
            object.checked_precise_translation()?;
        }
        let mut writer = BufferedCrcWriter::new();
        serde_json::to_writer_pretty(&mut writer, document)?;
        return Ok(writer.finish());
    }

    let workers = std::thread::available_parallelism()
        .map_or(4, std::num::NonZeroUsize::get)
        .min(8)
        .min(document.objects.len() / 128)
        .max(1);
    if workers <= 1 {
        for object in &document.objects {
            object.transform().matrix()?;
            object.checked_precise_translation()?;
        }
        let mut writer = BufferedCrcWriter::new();
        serde_json::to_writer_pretty(&mut writer, document)?;
        return Ok(writer.finish());
    }

    let prefix = format!(
        "{{\n  \"version\": {},\n  \"objects\": [\n",
        document.version
    );
    let chunk_size = document.objects.len().div_ceil(workers);
    let chunk_results: Vec<Result<(u32, usize), DocumentError>> = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        let mut offset = 0_usize;
        for slice in document.objects.chunks(chunk_size) {
            let start = offset;
            offset += slice.len();
            handles.push(scope.spawn(move || hash_object_slice(slice, start)));
        }
        handles
            .into_iter()
            .map(|h| h.join().expect("canonical CRC worker panicked"))
            .collect()
    });

    let mut crc = voxy_core::crc32c(prefix.as_bytes());
    for result in chunk_results {
        let (chunk_crc, chunk_len) = result?;
        crc = voxy_core::crc32c_combine(crc, chunk_crc, chunk_len);
    }
    Ok(voxy_core::crc32c_append(crc, b"\n  ]\n}"))
}
