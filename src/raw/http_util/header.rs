// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use base64::engine::general_purpose;
use base64::Engine;
use http::header::HeaderName;
use http::header::CONTENT_DISPOSITION;
use http::header::CONTENT_LENGTH;
use http::header::CONTENT_RANGE;
use http::header::CONTENT_TYPE;
use http::header::ETAG;
use http::header::LAST_MODIFIED;
use http::header::LOCATION;
use http::HeaderMap;
use md5::Digest;
use time::format_description::well_known::Rfc2822;
use time::OffsetDateTime;

use crate::raw::*;
use crate::EntryMode;
use crate::Error;
use crate::ErrorKind;
use crate::Metadata;
use crate::Result;

/// Parse redirect location from header map
///
/// # Note
/// The returned value maybe a relative path, like `/index.html`, `/robots.txt`, etc.
pub fn parse_location(headers: &HeaderMap) -> Result<Option<&str>> {
    match headers.get(LOCATION) {
        None => Ok(None),
        Some(v) => Ok(Some(v.to_str().map_err(|e| {
            Error::new(
                ErrorKind::Unexpected,
                "header value has to be valid utf-8 string",
            )
            .with_operation("http_util::parse_location")
            .set_source(e)
        })?)),
    }
}

/// Parse content length from header map.
pub fn parse_content_length(headers: &HeaderMap) -> Result<Option<u64>> {
    match headers.get(CONTENT_LENGTH) {
        None => Ok(None),
        Some(v) => Ok(Some(
            v.to_str()
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Unexpected,
                        "header value is not valid utf-8 string",
                    )
                    .with_operation("http_util::parse_content_length")
                    .set_source(e)
                })?
                .parse::<u64>()
                .map_err(|e| {
                    Error::new(ErrorKind::Unexpected, "header value is not valid integer")
                        .with_operation("http_util::parse_content_length")
                        .set_source(e)
                })?,
        )),
    }
}

/// Parse content md5 from header map.
pub fn parse_content_md5(headers: &HeaderMap) -> Result<Option<&str>> {
    match headers.get(HeaderName::from_static("content-md5")) {
        None => Ok(None),
        Some(v) => Ok(Some(v.to_str().map_err(|e| {
            Error::new(
                ErrorKind::Unexpected,
                "header value is not valid utf-8 string",
            )
            .with_operation("http_util::parse_content_md5")
            .set_source(e)
        })?)),
    }
}

/// Parse content type from header map.
pub fn parse_content_type(headers: &HeaderMap) -> Result<Option<&str>> {
    match headers.get(CONTENT_TYPE) {
        None => Ok(None),
        Some(v) => Ok(Some(v.to_str().map_err(|e| {
            Error::new(
                ErrorKind::Unexpected,
                "header value is not valid utf-8 string",
            )
            .with_operation("http_util::parse_content_type")
            .set_source(e)
        })?)),
    }
}

/// Parse content range from header map.
pub fn parse_content_range(headers: &HeaderMap) -> Result<Option<BytesContentRange>> {
    match headers.get(CONTENT_RANGE) {
        None => Ok(None),
        Some(v) => Ok(Some(
            v.to_str()
                .map_err(|e| {
                    Error::new(
                        ErrorKind::Unexpected,
                        "header value is not valid utf-8 string",
                    )
                    .with_operation("http_util::parse_content_range")
                    .set_source(e)
                })?
                .parse()?,
        )),
    }
}

/// Parse last modified from header map.
pub fn parse_last_modified(headers: &HeaderMap) -> Result<Option<OffsetDateTime>> {
    match headers.get(LAST_MODIFIED) {
        None => Ok(None),
        Some(v) => {
            let v = v.to_str().map_err(|e| {
                Error::new(
                    ErrorKind::Unexpected,
                    "header value is not valid utf-8 string",
                )
                .with_operation("http_util::parse_last_modified")
                .set_source(e)
            })?;
            let t = OffsetDateTime::parse(v, &Rfc2822).map_err(|e| {
                Error::new(
                    ErrorKind::Unexpected,
                    "header value is not valid rfc2822 time",
                )
                .with_operation("http_util::parse_last_modified")
                .set_source(e)
            })?;

            Ok(Some(t))
        }
    }
}

/// Parse etag from header map.
pub fn parse_etag(headers: &HeaderMap) -> Result<Option<&str>> {
    match headers.get(ETAG) {
        None => Ok(None),
        Some(v) => Ok(Some(v.to_str().map_err(|e| {
            Error::new(
                ErrorKind::Unexpected,
                "header value is not valid utf-8 string",
            )
            .with_operation("http_util::parse_etag")
            .set_source(e)
        })?)),
    }
}

/// Parse Content-Disposition for header map
pub fn parse_content_disposition(headers: &HeaderMap) -> Result<Option<&str>> {
    match headers.get(CONTENT_DISPOSITION) {
        None => Ok(None),
        Some(v) => Ok(Some(v.to_str().map_err(|e| {
            Error::new(
                ErrorKind::Unexpected,
                "header value has to be valid utf-8 string",
            )
            .with_operation("http_util::parse_content_disposition")
            .set_source(e)
        })?)),
    }
}

/// parse_into_metadata will parse standards http headers into Metadata.
///
/// # Notes
///
/// parse_into_metadata only handles the standard behavior of http
/// headers. If services have their own logic, they should update the parsed
/// metadata on demand.
pub fn parse_into_metadata(path: &str, headers: &HeaderMap) -> Result<Metadata> {
    let mode = if path.ends_with('/') {
        EntryMode::DIR
    } else {
        EntryMode::FILE
    };
    let mut m = Metadata::new(mode);

    if let Some(v) = parse_content_length(headers)? {
        m.set_content_length(v);
    }

    if let Some(v) = parse_content_type(headers)? {
        m.set_content_type(v);
    }

    if let Some(v) = parse_content_range(headers)? {
        m.set_content_range(v);
    }

    if let Some(v) = parse_etag(headers)? {
        m.set_etag(v);
    }

    if let Some(v) = parse_content_md5(headers)? {
        m.set_content_md5(v);
    }

    if let Some(v) = parse_last_modified(headers)? {
        m.set_last_modified(v);
    }

    if let Some(v) = parse_content_disposition(headers)? {
        m.set_content_disposition(v);
    }

    Ok(m)
}

/// format content md5 header by given input.
pub fn format_content_md5(bs: &[u8]) -> String {
    let mut hasher = md5::Md5::new();
    hasher.update(bs);

    general_purpose::STANDARD.encode(hasher.finalize())
}

/// format authorization header by basic auth.
///
/// # Errors
///
/// If input username is empty, function will return an unexpected error.
pub fn format_authorization_by_basic(username: &str, password: &str) -> Result<String> {
    if username.is_empty() {
        return Err(Error::new(
            ErrorKind::Unexpected,
            "can't build authorization header with empty username",
        ));
    }

    let value = general_purpose::STANDARD.encode(format!("{username}:{password}"));

    Ok(format!("Basic {value}"))
}

/// format authorization header by bearer token.
///
/// # Errors
///
/// If input token is empty, function will return an unexpected error.
pub fn format_authorization_by_bearer(token: &str) -> Result<String> {
    if token.is_empty() {
        return Err(Error::new(
            ErrorKind::Unexpected,
            "can't build authorization header with empty token",
        ));
    }

    Ok(format!("Bearer {token}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test cases is from https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html
    #[test]
    fn test_format_content_md5() {
        let cases = vec![(
            r#"<Delete>
<Object>
 <Key>sample1.txt</Key>
 </Object>
 <Object>
   <Key>sample2.txt</Key>
 </Object>
 </Delete>"#,
            "WOctCY1SS662e7ziElh4cw==",
        )];

        for (input, expected) in cases {
            let actual = format_content_md5(input.as_bytes());

            assert_eq!(actual, expected)
        }
    }

    /// Test cases is borrowed from
    ///
    /// - RFC2617: https://datatracker.ietf.org/doc/html/rfc2617#section-2
    /// - MDN: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Authorization
    #[test]
    fn test_format_authorization_by_basic() {
        let cases = vec![
            ("aladdin", "opensesame", "Basic YWxhZGRpbjpvcGVuc2VzYW1l"),
            ("aladdin", "", "Basic YWxhZGRpbjo="),
            (
                "Aladdin",
                "open sesame",
                "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==",
            ),
            ("Aladdin", "", "Basic QWxhZGRpbjo="),
        ];

        for (username, password, expected) in cases {
            let actual =
                format_authorization_by_basic(username, password).expect("format must success");

            assert_eq!(actual, expected)
        }
    }

    /// Test cases is borrowed from
    ///
    /// - RFC6750: https://datatracker.ietf.org/doc/html/rfc6750
    #[test]
    fn test_format_authorization_by_bearer() {
        let cases = vec![("mF_9.B5f-4.1JqM", "Bearer mF_9.B5f-4.1JqM")];

        for (token, expected) in cases {
            let actual = format_authorization_by_bearer(token).expect("format must success");

            assert_eq!(actual, expected)
        }
    }
}
