use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::ops::ControlFlow;

pub(crate) const BLOCK_SIZE_BYTES: usize = 64 * 1024;

pub(crate) trait JsonlRecord: BufRead {
    fn rewind(&mut self) -> io::Result<()>;
}

pub(crate) fn scan_jsonl_records_from_end<R, B, F>(
    reader: &mut R,
    mut inspect: F,
) -> io::Result<ControlFlow<B>>
where
    R: Read + Seek,
    F: FnMut(&mut dyn JsonlRecord) -> ControlFlow<B>,
{
    let mut reader = BufReader::with_capacity(BLOCK_SIZE_BYTES, reader);
    let mut position = reader.seek(SeekFrom::End(0))?;
    if position == 0 {
        return Ok(ControlFlow::Continue(()));
    }

    let file_end = position;
    let mut record_end = file_end;
    let mut terminated_by_newline = false;
    let mut saw_newline = false;
    let mut block = vec![0; BLOCK_SIZE_BYTES];

    while position > 0 {
        let read_size = position.min(BLOCK_SIZE_BYTES as u64) as usize;
        position -= read_size as u64;
        reader.seek(SeekFrom::Start(position))?;
        reader.read_exact(&mut block[..read_size])?;

        for (index, &byte) in block[..read_size].iter().enumerate().rev() {
            if byte != b'\n' {
                continue;
            }

            saw_newline = true;
            let record_start = position + index as u64 + 1;
            if record_start == file_end {
                record_end = file_end - 1;
                terminated_by_newline = true;
                continue;
            }

            if let ControlFlow::Break(value) = inspect_record(
                &mut reader,
                record_start,
                record_end,
                terminated_by_newline,
                &mut inspect,
            )? {
                return Ok(ControlFlow::Break(value));
            }

            record_end = record_start - 1;
            terminated_by_newline = true;
        }
    }

    if record_end > 0 || saw_newline {
        if let ControlFlow::Break(value) = inspect_record(
            &mut reader,
            0,
            record_end,
            terminated_by_newline,
            &mut inspect,
        )? {
            return Ok(ControlFlow::Break(value));
        }
    }

    Ok(ControlFlow::Continue(()))
}

fn inspect_record<R, B, F>(
    reader: &mut R,
    record_start: u64,
    record_end: u64,
    terminated_by_newline: bool,
    inspect: &mut F,
) -> io::Result<ControlFlow<B>>
where
    R: BufRead + Seek,
    F: FnMut(&mut dyn JsonlRecord) -> ControlFlow<B>,
{
    let mut content_end = record_end;
    if terminated_by_newline && content_end > record_start {
        reader.seek(SeekFrom::Start(content_end - 1))?;
        let mut last_byte = [0];
        reader.read_exact(&mut last_byte)?;
        if last_byte[0] == b'\r' {
            content_end -= 1;
        }
    }

    reader.seek(SeekFrom::Start(record_start))?;
    let length = content_end - record_start;
    let mut record = BoundedRecord {
        reader: reader.by_ref().take(length),
        start: record_start,
        length,
    };
    Ok(inspect(&mut record))
}

struct BoundedRecord<'a, R> {
    reader: io::Take<&'a mut R>,
    start: u64,
    length: u64,
}

impl<R: Read> Read for BoundedRecord<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buffer)
    }
}

impl<R: BufRead> BufRead for BoundedRecord<'_, R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.reader.consume(amount);
    }
}

impl<R: BufRead + Seek> JsonlRecord for BoundedRecord<'_, R> {
    fn rewind(&mut self) -> io::Result<()> {
        self.reader.get_mut().seek(SeekFrom::Start(self.start))?;
        self.reader.set_limit(self.length);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read, Seek, SeekFrom};
    use std::ops::ControlFlow;

    use super::{scan_jsonl_records_from_end, BLOCK_SIZE_BYTES};
    use crate::transcript_record_probe::{has_tool_result, has_type};

    #[test]
    fn scans_trailing_newline_newest_first() {
        assert_eq!(collect_lines(b"first\nsecond\n"), ["second", "first"]);
    }

    #[test]
    fn scans_last_line_without_newline() {
        assert_eq!(collect_lines(b"first\nsecond"), ["second", "first"]);
    }

    #[test]
    fn strips_crlf_delimiters() {
        assert_eq!(collect_lines(b"first\r\nsecond\r\n"), ["second", "first"]);
    }

    #[test]
    fn does_not_emit_a_line_for_an_empty_file() {
        assert!(collect_lines(b"").is_empty());
    }

    #[test]
    fn handles_short_reads() {
        let mut reader = ShortReader {
            inner: Cursor::new(b"first\nsecond\nthird".to_vec()),
            max_read: 3,
        };
        let mut lines = Vec::new();

        let _ = scan_jsonl_records_from_end(&mut reader, |record| {
            let mut line = String::new();
            record.read_to_string(&mut line).unwrap();
            lines.push(line);
            ControlFlow::<()>::Continue(())
        })
        .unwrap();

        assert_eq!(lines, ["third", "second", "first"]);
    }

    #[test]
    fn scans_a_line_across_multiple_blocks() {
        let long_line = "x".repeat(BLOCK_SIZE_BYTES * 2 + 17);
        let data = format!("first\n{long_line}\nlast\n");

        assert_eq!(
            collect_lines(data.as_bytes()),
            ["last", &long_line, "first"]
        );
    }

    #[test]
    fn streams_a_line_larger_than_the_read_buffer() {
        let long_line = "x".repeat(BLOCK_SIZE_BYTES * 4);
        let data = format!("{long_line}\n");
        let mut reader = Cursor::new(data.as_bytes());

        let result = scan_jsonl_records_from_end(&mut reader, |record| {
            assert_eq!(record.fill_buf().unwrap().len(), BLOCK_SIZE_BYTES);
            ControlFlow::Break(())
        })
        .unwrap();

        assert_eq!(result, ControlFlow::Break(()));
    }

    #[test]
    fn probes_tool_result_before_reading_its_content() {
        let content = "x".repeat(BLOCK_SIZE_BYTES * 8);
        let data = format!(
            r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","content":"{content}"}}]}}}}"#
        );
        let mut reader = CountingReader {
            inner: Cursor::new(data.as_bytes()),
            bytes_read: 0,
        };

        let result = scan_jsonl_records_from_end(&mut reader, |record| {
            assert!(has_type(record, "user"));
            record.rewind().unwrap();
            assert!(has_tool_result(record));
            ControlFlow::Break(())
        })
        .unwrap();

        assert_eq!(result, ControlFlow::Break(()));
        assert!(reader.bytes_read < data.len() + BLOCK_SIZE_BYTES * 3);
    }

    #[test]
    fn skips_invalid_utf8_lines() {
        let mut data = b"first\n".to_vec();
        data.extend_from_slice(&[0xff, 0xfe, b'\n']);
        data.extend_from_slice(b"last\n");

        assert_eq!(collect_lines(&data), ["last", "first"]);
    }

    #[test]
    fn stops_after_a_match_in_the_tail_block() {
        let mut data = vec![b'x'; BLOCK_SIZE_BYTES * 64];
        data.extend_from_slice(b"\n{\"target\":true}\n");
        let mut reader = CountingReader {
            inner: Cursor::new(data),
            bytes_read: 0,
        };

        let result = scan_jsonl_records_from_end(&mut reader, |record| {
            let mut line = String::new();
            record.read_to_string(&mut line).unwrap();
            if line == r#"{"target":true}"# {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .unwrap();

        assert_eq!(result, ControlFlow::Break(()));
        assert!(reader.bytes_read < BLOCK_SIZE_BYTES * 2);
    }

    fn collect_lines(data: &[u8]) -> Vec<String> {
        let mut reader = Cursor::new(data.to_vec());
        let mut lines = Vec::new();
        let _ = scan_jsonl_records_from_end(&mut reader, |record| {
            let mut line = String::new();
            if record.read_to_string(&mut line).is_ok() {
                lines.push(line);
            }
            ControlFlow::<()>::Continue(())
        })
        .unwrap();
        lines
    }

    struct ShortReader<R> {
        inner: R,
        max_read: usize,
    }

    impl<R: Read> Read for ShortReader<R> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let limit = buffer.len().min(self.max_read);
            self.inner.read(&mut buffer[..limit])
        }
    }

    impl<R: Seek> Seek for ShortReader<R> {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.inner.seek(position)
        }
    }

    struct CountingReader<R> {
        inner: R,
        bytes_read: usize,
    }

    impl<R: Read> Read for CountingReader<R> {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let read = self.inner.read(buffer)?;
            self.bytes_read += read;
            Ok(read)
        }
    }

    impl<R: Seek> Seek for CountingReader<R> {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.inner.seek(position)
        }
    }
}
