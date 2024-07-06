use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use tracing::debug;
use url::Url;

use crate::{
    error::{bail, ensure, Result},
    player::{PlayerTrait, WebPlayer},
};

thread_local!(
    static URL_ALIASES: RefCell<HashMap<String, String>> = RefCell::default();
);

/// clear aliases
pub fn on_new_map_loaded() {
    debug!("url_aliases on_new_map_loaded");

    URL_ALIASES.with(move |cell| {
        let url_aliases = &mut *cell.borrow_mut();
        url_aliases.clear();
    });
}

pub fn add_alias(alias: &str, url: &str) -> Result<()> {
    ensure!(!alias.is_empty(), "alias is empty");

    // only allow in the format of url schemes
    let Ok(alias_url) = Url::parse(&format!("{alias}:")) else {
        bail!("alias must only contain alphanumeric or - or _ characters");
    };
    if alias != alias_url.scheme() {
        // TODO better error?
        bail!("alias must only contain alphanumeric or - or _ characters");
    }

    // make sure it's a normal url
    WebPlayer::from_input(url)?;

    URL_ALIASES.with(move |cell| {
        let url_aliases = &mut *cell.borrow_mut();
        url_aliases.insert(alias.to_string(), url.to_string());
    });

    Ok(())
}

pub fn resolve_alias_url(alias_url: &str) -> Result<String> {
    ensure!(!alias_url.is_empty(), "url is empty");

    let mut alias_url = alias_url.to_string();
    if !alias_url.contains(':') {
        alias_url = format!("{alias_url}:");
    }

    let alias_url: Url = alias_url.parse()?;
    let alias = alias_url.scheme();

    let base_url = URL_ALIASES
        .with(move |cell| {
            let url_aliases = &*cell.borrow();
            url_aliases.get(alias).cloned()
        })
        .ok_or_else(|| format!("no alias found for {alias:?}"))?;

    let mut url: Url = base_url.parse()?;
    // println!("url = {url:#?}");
    // println!("alias_url = {alias_url:#?}");
    url.set_path(&format!("{}{}", url.path(), alias_url.path()));
    // println!("url = {url:#?}");

    // let query = match (url.query(), alias_url.query()) {
    //     (None, None) => None,
    //     (None, Some(query)) | (Some(query), None) => Some(query.to_string()),
    //     (Some(left), Some(right)) => match (left, right) {
    //         ("", query) | (query, "") => Some(query.to_string()),
    //         (left, right) => {
    //             if left.is_empty() {
    //                 Some(right.to_string())
    //             } else {
    //                 Some(format!("{left}&{right}"))
    //             }
    //         }
    //     },
    // };
    if url.query().is_some() || alias_url.query().is_some() {
        let new_query_keys = alias_url
            .query_pairs()
            .map(|(key, _)| key.to_string())
            .collect::<HashSet<_>>();
        let prev_query_pairs = url
            .query_pairs()
            .filter(|(key, _)| !new_query_keys.contains(key.as_ref()))
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<Vec<_>>();
        let mut query_pairs_mut = url.query_pairs_mut();
        query_pairs_mut.clear();
        for (key, value) in prev_query_pairs {
            if value.is_empty() {
                query_pairs_mut.append_key_only(&key);
            } else {
                query_pairs_mut.append_pair(&key, &value);
            }
        }
        for (key, value) in alias_url.query_pairs() {
            if value.is_empty() {
                query_pairs_mut.append_key_only(&key);
            } else {
                query_pairs_mut.append_pair(&key, &value);
            }
        }
    }

    let url = url.as_str().to_string();
    // println!("url = {url:#?}");

    Ok(url)
}

pub fn get_all_aliases() -> HashMap<String, String> {
    URL_ALIASES.with(move |cell| {
        let url_aliases = &mut *cell.borrow_mut();
        url_aliases.clone()
    })
}

#[test]
fn test_add_alias() {
    for (alias, url, expected) in [
        ("example", "https://example.com/path", true),
        //
        ("asdf0", "https://example.com/path", true),
        ("0", "https://example.com/path", false),
        ("0asdf", "https://example.com/path", false),
        //
        ("example:", "https://example.com/path", false),
        ("example://", "https://example.com/path", false),
        ("example\\", "https://example.com/path", false),
        ("example/", "https://example.com/path", false),
        ("example!", "https://example.com/path", false),
        ("example:okay", "https://example.com/path", false),
        //
        ("example", "example.com/path", false),
        ("example", "example:url", false),
        ("example", "example:", false),
        ("example", "example", false),
        //
        ("example", "https://example.com/path?with=query", true),
    ] {
        let result = add_alias(alias, url);
        assert_eq!(result.is_ok(), expected, "{alias:?}: {result:?}");
    }
}

#[test]
fn test_resolve_alias_url() {
    add_alias("example", "https://example.com/path").unwrap();
    add_alias("with-path", "https://example.com/path/").unwrap();
    add_alias("with-query", "https://example.com/?with=query").unwrap();
    add_alias(
        "with-path-and-query",
        "https://example.com/path/?with=query",
    )
    .unwrap();
    add_alias("strange", "https://example.com/?strange").unwrap();

    for (alias_url, expected) in [
        ("example", Some("https://example.com/path")),
        ("example:", Some("https://example.com/path")),
        ("example:okay", Some("https://example.com/pathokay")),
        ("example:/okay", Some("https://example.com/path/okay")),
        ("with-path:okay", Some("https://example.com/path/okay")),
        (
            "example:/okay/more-path",
            Some("https://example.com/path/okay/more-path"),
        ),
        ("noon", None),
        ("https://example.com/", None),
        //
        ("with-query", Some("https://example.com/?with=query")),
        ("with-query:", Some("https://example.com/?with=query")),
        (
            "with-query:?with=overwritten",
            Some("https://example.com/?with=overwritten"),
        ),
        (
            "with-query:pathed?and=another&more=more",
            Some("https://example.com/pathed?with=query&and=another&more=more"),
        ),
        (
            "with-path-and-query:pathed?and=another&more=more",
            Some("https://example.com/path/pathed?with=query&and=another&more=more"),
        ),
        //
        (
            "with-query:?strange",
            Some("https://example.com/?with=query&strange"),
        ),
        (
            "strange:?key=value",
            Some("https://example.com/?strange&key=value"),
        ),
        (
            "strange:?more-strange",
            Some("https://example.com/?strange&more-strange"),
        ),
    ] {
        let resolved = resolve_alias_url(alias_url);
        if let Some(expected) = expected {
            assert!(resolved.is_ok(), "not is_ok: {alias_url:?}: {resolved:?}");
            let resolved = resolved.unwrap();
            assert_eq!(resolved, expected, "{alias_url:?}");
        } else {
            assert!(resolved.is_err(), "not is_err: {alias_url:?}: {resolved:?}");
        }
    }
}
