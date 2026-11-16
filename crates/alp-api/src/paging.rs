//! Cursor pagination for `alp-mcp` list tools and the API list methods behind
//! them.
//!
//! MCP places a size cap on what one response may return, so list tools expose
//! `limit` and `cursor` arguments. A cursor is the opaque position of the next
//! item, encoded so clients treat it as a token rather than an index they can
//! compute. The page also reports `total` and, when more items remain,
//! `next_cursor`.
//!
//! Pagination is applied to the API's own ordered list, not to a snapshot: if
//! the project changes between pages, the next page continues from the same
//! offset into the new list. That is the same trade-off every offset cursor
//! makes. Writes bump the project `revision`, so a caller that needs a stable
//! sequence can compare `revision` between pages.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;

/// Default page size when a caller does not pass `limit`.
pub const DEFAULT_PAGE_SIZE: usize = 50;

/// Largest page a single response will return.
pub const MAX_PAGE_SIZE: usize = 200;

/// Version prefix on every cursor this module emits.
const CURSOR_PREFIX: &str = "v1";

/// One page of an ordered list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Items in this page, in list order.
    pub items: Vec<T>,
    /// Total number of items in the underlying list.
    pub total: usize,
    /// Offset of the first item in this page.
    pub offset: usize,
    /// Cursor for the next page, absent on the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl<T> Page<T> {
    /// Whether this page is the last one.
    #[must_use]
    pub fn is_last(&self) -> bool {
        self.next_cursor.is_none()
    }
}

/// A resolved page request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// First item to return.
    pub offset: usize,
    /// Maximum items to return.
    pub limit: usize,
}

impl PageRequest {
    /// Resolve `limit`/`cursor` arguments into an offset and a bounded limit.
    ///
    /// An absent `limit` uses [`DEFAULT_PAGE_SIZE`]; a value above
    /// [`MAX_PAGE_SIZE`] is clamped rather than rejected so a client asking for
    /// "everything" still gets a well-formed page. A malformed cursor is a
    /// validation error, because silently restarting at offset 0 would make a
    /// caller loop forever.
    pub fn resolve(limit: Option<u64>, cursor: Option<&str>) -> Result<Self, ApiError> {
        let offset = match cursor {
            Some(token) => decode_cursor(token)?,
            None => 0,
        };
        let requested = limit.unwrap_or(DEFAULT_PAGE_SIZE as u64);
        if requested == 0 {
            return Err(ApiError::Validation(
                "limit must be at least 1; omit it to use the default page size".into(),
            ));
        }
        let limit = usize::try_from(requested)
            .unwrap_or(MAX_PAGE_SIZE)
            .min(MAX_PAGE_SIZE);
        Ok(Self { offset, limit })
    }
}

/// Encode an offset as an opaque cursor token.
#[must_use]
pub fn encode_cursor(offset: usize) -> String {
    format!("{CURSOR_PREFIX}{offset:x}")
}

/// Decode a cursor token back into an offset.
pub fn decode_cursor(token: &str) -> Result<usize, ApiError> {
    let invalid = || {
        ApiError::Validation(
            "invalid cursor: pass the `next_cursor` value from the previous page, or omit it to start over"
                .into(),
        )
    };
    token
        .trim()
        .strip_prefix(CURSOR_PREFIX)
        .and_then(|hex| usize::from_str_radix(hex, 16).ok())
        .ok_or_else(invalid)
}

/// Slice an ordered list into one page.
///
/// An offset past the end yields an empty page rather than an error: the caller
/// asked for a position that no longer exists, and an empty page with `total`
/// tells it exactly that.
#[must_use]
pub fn paginate<T>(items: Vec<T>, request: PageRequest) -> Page<T> {
    let total = items.len();
    let start = request.offset.min(total);
    let end = start.saturating_add(request.limit).min(total);
    let page_items: Vec<T> = items.into_iter().skip(start).take(end - start).collect();
    let next_cursor = if end < total {
        Some(encode_cursor(end))
    } else {
        None
    };
    Page {
        items: page_items,
        total,
        offset: start,
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors_round_trip_and_reject_junk() {
        assert_eq!(decode_cursor(&encode_cursor(0)).expect("zero"), 0);
        assert_eq!(decode_cursor(&encode_cursor(173)).expect("offset"), 173);
        for junk in ["", "v1", "v1zz", "offset:3", "3", "v2a"] {
            assert!(
                decode_cursor(junk).is_err(),
                "'{junk}' must not decode to an offset"
            );
        }
    }

    #[test]
    fn pages_walk_the_whole_list_exactly_once() {
        let items: Vec<usize> = (0..7).collect();
        let mut cursor: Option<String> = None;
        let mut seen = Vec::new();
        loop {
            let request = PageRequest::resolve(Some(3), cursor.as_deref()).expect("page request");
            let page = paginate(items.clone(), request);
            assert_eq!(page.total, 7);
            seen.extend(page.items);
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        assert_eq!(seen, items, "every item appears exactly once, in order");
    }

    #[test]
    fn limits_are_clamped_and_zero_is_rejected() {
        let request = PageRequest::resolve(None, None).expect("default");
        assert_eq!(request.limit, DEFAULT_PAGE_SIZE);
        assert_eq!(request.offset, 0);

        let clamped = PageRequest::resolve(Some(10_000), None).expect("clamped");
        assert_eq!(clamped.limit, MAX_PAGE_SIZE);

        assert!(PageRequest::resolve(Some(0), None).is_err());
    }

    #[test]
    fn an_offset_past_the_end_is_an_empty_page_not_an_error() {
        let page = paginate(
            vec![1, 2, 3],
            PageRequest {
                offset: 99,
                limit: 10,
            },
        );
        assert!(page.items.is_empty());
        assert_eq!(page.total, 3);
        assert_eq!(page.offset, 3);
        assert!(page.is_last());
    }

    #[test]
    fn a_full_final_page_has_no_cursor() {
        let page = paginate(
            vec![1, 2, 3],
            PageRequest {
                offset: 0,
                limit: 3,
            },
        );
        assert_eq!(page.items.len(), 3);
        assert!(
            page.is_last(),
            "an exactly-full last page must not offer a cursor"
        );
    }
}
