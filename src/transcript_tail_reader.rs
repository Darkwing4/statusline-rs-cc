use std::io::{self, Read, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::str;

pub(crate) const BLOCK_SIZE_BYTES: usize = 64 * 1024;

pub(crate) fn scan_jsonl_lines_from_end<R, B, F>(
    reader: &mut R,
    mut inspect: F,
) -> io::Result<ControlFlow<B>>
where
    R: Read + Seek,
    F: FnMut(&str) -> ControlFlow<B>,
{
    let mut position = reader.seek(SeekFrom::End(0))?;
    if position == 0 {
        return Ok(ControlFlow::Continue(()));
    }

    let mut block = vec![0; BLOCK_SIZE_BYTES];
    let mut reversed_line = Vec::new();
    let mut at_end = true;
    let mut terminated_by_newline = false;
    let mut saw_newline = false;

    while position > 0 {
        let read_size = position.min(BLOCK_SIZE_BYTES as u64) as usize;
        position -= read_size as u64;
        reader.seek(SeekFrom::Start(position))?;
        reader.read_exact(&mut block[..read_size])?;

        for &byte in block[..read_size].iter().rev() {
            if byte != b'\n' {
                reversed_line.push(byte);
                at_end = false;
                continue;
            }

            saw_newline = true;
            if at_end && reversed_line.is_empty() {
                at_end = false;
                terminated_by_newline = true;
                continue;
            }

            if let ControlFlow::Break(value) =
                inspect_reversed_line(&mut reversed_line, terminated_by_newline, &mut inspect)
            {
                return Ok(ControlFlow::Break(value));
            }

            terminated_by_newline = true;
            at_end = false;
        }
    }

    if !reversed_line.is_empty() || saw_newline {
        if let ControlFlow::Break(value) =
            inspect_reversed_line(&mut reversed_line, terminated_by_newline, &mut inspect)
        {
            return Ok(ControlFlow::Break(value));
        }
    }

    Ok(ControlFlow::Continue(()))
}

fn inspect_reversed_line<B, F>(
    reversed_line: &mut Vec<u8>,
    terminated_by_newline: bool,
    inspect: &mut F,
) -> ControlFlow<B>
where
    F: FnMut(&str) -> ControlFlow<B>,
{
    reversed_line.reverse();
    if terminated_by_newline && reversed_line.last() == Some(&b'\r') {
        reversed_line.pop();
    }

    let result = str::from_utf8(reversed_line)
        .map(inspect)
        .unwrap_or(ControlFlow::Continue(()));
    reversed_line.clear();
    result
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read, Seek, SeekFrom};
    use std::ops::ControlFlow;

    use super::{scan_jsonl_lines_from_end, BLOCK_SIZE_BYTES};

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

        let _ = scan_jsonl_lines_from_end(&mut reader, |line| {
            lines.push(line.to_owned());
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

        let result = scan_jsonl_lines_from_end(&mut reader, |line| {
            if line == r#"{"target":true}"# {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .unwrap();

        assert_eq!(result, ControlFlow::Break(()));
        assert!(reader.bytes_read <= BLOCK_SIZE_BYTES);
    }

    fn collect_lines(data: &[u8]) -> Vec<String> {
        let mut reader = Cursor::new(data.to_vec());
        let mut lines = Vec::new();
        let _ = scan_jsonl_lines_from_end(&mut reader, |line| {
            lines.push(line.to_owned());
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
