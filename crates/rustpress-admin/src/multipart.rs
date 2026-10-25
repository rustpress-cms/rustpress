//! `multipart/form-data` bodies: the fields and the uploaded files.

/// A file field of a form.
#[derive(Debug, Clone)]
pub struct FilePart {
    pub field: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// The `boundary` parameter of a `Content-Type` header.
pub fn boundary(content_type: &str) -> Option<String> {
    let lower = content_type.to_lowercase();
    if !lower.starts_with("multipart/form-data") {
        return None;
    }
    let start = lower.find("boundary=")? + "boundary=".len();
    let rest = &content_type[start..];
    let value = rest.split(';').next()?.trim().trim_matches('"');
    (!value.is_empty()).then(|| value.to_owned())
}

fn header_param(headers: &str, name: &str) -> Option<String> {
    let marker = format!("{name}=\"");
    let start = headers.find(&marker)? + marker.len();
    let end = headers[start..].find('"')?;
    Some(headers[start..start + end].to_owned())
}

/// Splits a body into `(fields, files)`.
pub fn parse(boundary: &str, body: &[u8]) -> (Vec<(String, String)>, Vec<FilePart>) {
    let delimiter = format!("--{boundary}").into_bytes();
    let mut fields = Vec::new();
    let mut files = Vec::new();
    let mut position = match find(body, &delimiter, 0) {
        Some(p) => p + delimiter.len(),
        None => return (fields, files),
    };
    loop {
        // After a delimiter: `--` ends the body, otherwise CRLF and the part follows.
        if body[position..].starts_with(b"--") {
            break;
        }
        if body[position..].starts_with(b"\r\n") {
            position += 2;
        }
        let Some(header_end) = find(body, b"\r\n\r\n", position) else { break };
        let headers = String::from_utf8_lossy(&body[position..header_end]).into_owned();
        let data_start = header_end + 4;
        let Some(next) = find(body, &delimiter, data_start) else { break };
        // The data ends before the CRLF in front of the next delimiter.
        let data_end = next.saturating_sub(2).max(data_start);
        let data = &body[data_start..data_end];
        let name = header_param(&headers, "name").unwrap_or_default();
        match header_param(&headers, "filename") {
            Some(filename) => {
                let content_type = headers.lines().find_map(|l| l.to_lowercase().strip_prefix("content-type:").map(|v| v.trim().to_owned())).unwrap_or_default();
                if !filename.is_empty() {
                    files.push(FilePart { field: name, filename, content_type, bytes: data.to_vec() });
                }
            }
            None => fields.push((name, String::from_utf8_lossy(data).into_owned())),
        }
        position = next + delimiter.len();
        if position >= body.len() {
            break;
        }
    }
    (fields, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_files_are_split() {
        let body = b"--XX\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n--XX\r\nContent-Disposition: form-data; name=\"f\"; filename=\"x.txt\"\r\nContent-Type: text/plain\r\n\r\nhello\r\n--XX--\r\n";
        let (fields, files) = parse("XX", body);
        assert_eq!(fields, vec![("a".to_owned(), "1".to_owned())]);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "x.txt");
        assert_eq!(files[0].bytes, b"hello");
        assert_eq!(files[0].content_type, "text/plain");
    }

    #[test]
    fn boundary_is_read_from_the_header() {
        assert_eq!(boundary("multipart/form-data; boundary=----abc").as_deref(), Some("----abc"));
        assert_eq!(boundary("application/x-www-form-urlencoded"), None);
    }
}
