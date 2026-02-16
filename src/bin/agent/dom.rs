use anyhow::Result;
use headless_chrome::Tab;
use std::sync::Arc;

use crate::types::DOM_SNAPSHOT_MAX_CHARS;

/// JavaScript injected into the page to produce a simplified DOM representation.
/// NON-DESTRUCTIVE: reads the DOM without modifying styles or layout.
///
/// The script:
///   1. Skips script, style, noscript, svg elements (does NOT remove them).
///   2. Walks the visible DOM tree (max depth 15).
///   3. Assigns sequential IDs [e0], [e1], ... to interactive elements
///      (a, button, input, textarea, select) via data-eid attributes.
///   4. Emits a compact one-line-per-element text representation.
const SNAPSHOT_JS: &str = r#"
(() => {
  const SKIP = new Set(['SCRIPT','STYLE','NOSCRIPT','SVG','LINK']);
  let id = 0;
  const lines = [];
  const seen = new Set();

  function isVisible(el) {
    if (el.offsetParent === null && el.tagName !== 'BODY' && el.tagName !== 'HTML') return false;
    const s = getComputedStyle(el);
    return s.display !== 'none' && s.visibility !== 'hidden' && s.opacity !== '0';
  }

  function walk(node, depth) {
    if (depth > 15) return;
    for (const child of node.children) {
      if (SKIP.has(child.tagName)) continue;
      if (!isVisible(child)) continue;
      const tag = child.tagName.toLowerCase();
      const interactive = ['a','button','input','textarea','select'].includes(tag);

      if (interactive) {
        const eid = '[e' + (id++) + ']';
        child.setAttribute('data-eid', eid);
        let desc = '';
        if (tag === 'a') {
          desc = eid + ' link "' + (child.textContent||'').trim().slice(0,60) + '"';
        } else if (tag === 'input' || tag === 'textarea') {
          desc = eid + ' ' + tag + ' type=' + (child.type||'text') + ' placeholder="' + (child.placeholder||'') + '"';
          if (child.name) desc += ' name=' + child.name;
          if (child.value) desc += ' value="' + child.value.slice(0,30) + '"';
        } else if (tag === 'button') {
          desc = eid + ' button "' + (child.textContent||'').trim().slice(0,60) + '"';
        } else if (tag === 'select') {
          const opts = [...child.options].map(o => o.text.trim().slice(0,20)).join('|');
          desc = eid + ' select [' + opts + ']';
        }
        if (desc && !seen.has(desc)) {
          seen.add(desc);
          lines.push(desc);
        }
      } else {
        const text = child.textContent ? child.textContent.trim() : '';
        if (text && text.length > 2 && text.length < 200 && child.children.length === 0) {
          const t = text.slice(0, 100);
          if (!seen.has(t)) {
            seen.add(t);
            lines.push('  "' + t + '"');
          }
        }
      }
      walk(child, depth + 1);
    }
  }

  walk(document.body, 0);
  return lines.join('\n');
})()
"#;

/// Capture a simplified DOM snapshot from the current page.
pub fn capture_dom_snapshot(tab: &Arc<Tab>) -> Result<String> {
    let result = tab.evaluate(SNAPSHOT_JS, false)?;
    let raw = result
        .value
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    if raw.len() > DOM_SNAPSHOT_MAX_CHARS {
        Ok(format!(
            "{}\n... [truncated, {} total chars]",
            &raw[..DOM_SNAPSHOT_MAX_CHARS],
            raw.len()
        ))
    } else {
        Ok(raw)
    }
}

/// Get the current page URL.
pub fn get_current_url(tab: &Arc<Tab>) -> Result<String> {
    let result = tab.evaluate("window.location.href", false)?;
    Ok(result
        .value
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "unknown".to_string()))
}

/// Get the current page title.
pub fn get_page_title(tab: &Arc<Tab>) -> Result<String> {
    let result = tab.evaluate("document.title", false)?;
    Ok(result
        .value
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "untitled".to_string()))
}
