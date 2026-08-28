use std::io::{BufRead, BufReader, Read};

use crate::transcript_tail_reader::BLOCK_SIZE_BYTES;

const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn read_records_forward<R, F>(reader: R, mut visit: F) -> u64
where
    R: Read,
    F: FnMut(&[u8]),
{
    let mut buffered = BufReader::with_capacity(BLOCK_SIZE_BYTES, reader);
    let mut record = Vec::new();
    let mut consumed = 0;

    loop {
        record.clear();
        let Ok(read) = buffered.read_until(b'\n', &mut record) else {
            break;
        };

        if read == 0 {
            break;
        }

        if record.last() != Some(&b'\n') {
            break;
        }

        consumed += read as u64;
        let body = trim_delimiters(&record);

        if body.is_empty() || body.len() > MAX_RECORD_BYTES {
            continue;
        }

        visit(body);
    }

    consumed
}

fn trim_delimiters(record: &[u8]) -> &[u8] {
    let mut end = record.len();

    while end > 0 && (record[end - 1] == b'\n' || record[end - 1] == b'\r') {
        end -= 1;
    }

    &record[..end]
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{read_records_forward, MAX_RECORD_BYTES};

    fn collect(data: &[u8]) -> (Vec<String>, u64) {
        let mut records = Vec::new();
        let consumed = read_records_forward(Cursor::new(data.to_vec()), |record| {
            records.push(String::from_utf8_lossy(record).into_owned());
        });

        (records, consumed)
    }

    #[test]
    fn reads_complete_records_and_strips_delimiters() {
        let (records, consumed) = collect(b"first\r\nsecond\n");

        assert_eq!(records, ["first", "second"]);
        assert_eq!(consumed, 14);
    }

    #[test]
    fn stops_before_a_record_that_is_still_being_written() {
        let (records, consumed) = collect(b"first\nhalf-writ");

        assert_eq!(records, ["first"]);
        assert_eq!(consumed, 6);
    }

    #[test]
    fn skips_blank_and_oversized_records_but_still_counts_them() {
        let huge = vec![b'x'; MAX_RECORD_BYTES + 1];
        let mut data = b"\n".to_vec();
        data.extend_from_slice(&huge);
        data.push(b'\n');
        data.extend_from_slice(b"tail\n");

        let (records, consumed) = collect(&data);

        assert_eq!(records, ["tail"]);
        assert_eq!(consumed, data.len() as u64);
    }
}
