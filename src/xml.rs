//! Minimal XML writer with proper escaping and indentation.
//!
//! Not a general-purpose XML library; just enough to build QGIS project
//! files. Every element starts on its own line, indented by its depth.

/// Builds an XML fragment. `base_depth` is the indentation depth (2 spaces
/// per level) at which the first element is written, so fragments can be
/// spliced into the project template at the right indentation.
pub(crate) struct XmlWriter {
    buf: String,
    stack: Vec<Frame>,
    base_depth: usize,
}

struct Frame {
    tag: String,
    /// The start tag is not closed yet (attributes may still be added).
    open: bool,
    has_children: bool,
    has_text: bool,
}

impl XmlWriter {
    pub(crate) fn new(base_depth: usize) -> Self {
        XmlWriter {
            buf: String::new(),
            stack: Vec::new(),
            base_depth,
        }
    }

    fn newline_indent(&mut self) {
        self.buf.push('\n');
        for _ in 0..(self.base_depth + self.stack.len()) {
            self.buf.push_str("  ");
        }
    }

    /// Close the pending start tag of the current element, if any.
    fn close_start_tag(&mut self) {
        if let Some(top) = self.stack.last_mut()
            && top.open
        {
            self.buf.push('>');
            top.open = false;
        }
    }

    /// Start a new element. Attributes can be added with [`Self::attr`]
    /// until any other content is written.
    pub(crate) fn start(&mut self, tag: &str) -> &mut Self {
        self.close_start_tag();
        if let Some(top) = self.stack.last_mut() {
            top.has_children = true;
        }
        self.newline_indent();
        self.buf.push('<');
        self.buf.push_str(tag);
        self.stack.push(Frame {
            tag: tag.to_string(),
            open: true,
            has_children: false,
            has_text: false,
        });
        self
    }

    /// Add an attribute to the most recently started element.
    pub(crate) fn attr(&mut self, key: &str, value: impl std::fmt::Display) -> &mut Self {
        debug_assert!(self.stack.last().is_some_and(|f| f.open));
        self.buf.push(' ');
        self.buf.push_str(key);
        self.buf.push_str("=\"");
        escape_attr(&mut self.buf, &value.to_string());
        self.buf.push('"');
        self
    }

    /// Write escaped text content into the current element.
    pub(crate) fn text(&mut self, text: &str) -> &mut Self {
        self.close_start_tag();
        if let Some(top) = self.stack.last_mut() {
            top.has_text = true;
        }
        escape_text(&mut self.buf, text);
        self
    }

    /// End the current element.
    pub(crate) fn end(&mut self) -> &mut Self {
        let frame = self.stack.pop().expect("end() without start()");
        if frame.open {
            self.buf.push_str("/>");
        } else if frame.has_text && !frame.has_children {
            self.buf.push_str("</");
            self.buf.push_str(&frame.tag);
            self.buf.push('>');
        } else {
            self.newline_indent();
            self.buf.push_str("</");
            self.buf.push_str(&frame.tag);
            self.buf.push('>');
        }
        self
    }

    /// Write `<tag>text</tag>` in one call.
    pub(crate) fn elem(&mut self, tag: &str, text: &str) -> &mut Self {
        self.start(tag).text(text).end()
    }

    /// Write an empty element with the given attributes.
    pub(crate) fn empty(&mut self, tag: &str, attrs: &[(&str, &str)]) -> &mut Self {
        self.start(tag);
        for (k, v) in attrs {
            self.attr(k, v);
        }
        self.end()
    }

    /// Consume the writer and return the fragment.
    pub(crate) fn finish(self) -> String {
        assert!(self.stack.is_empty(), "unclosed XML elements");
        self.buf
    }
}

pub(crate) fn escape_attr(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            '"' => buf.push_str("&quot;"),
            _ => buf.push(c),
        }
    }
}

pub(crate) fn escape_text(buf: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '&' => buf.push_str("&amp;"),
            '<' => buf.push_str("&lt;"),
            '>' => buf.push_str("&gt;"),
            _ => buf.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_elements() {
        let mut w = XmlWriter::new(0);
        w.start("a")
            .attr("x", "1");
        w.elem("b", "hello");
        w.empty("c", &[("y", "2")]);
        w.end();
        assert_eq!(
            w.finish(),
            "\n<a x=\"1\">\n  <b>hello</b>\n  <c y=\"2\"/>\n</a>"
        );
    }

    #[test]
    fn escaping() {
        let mut w = XmlWriter::new(0);
        w.start("a")
            .attr("k", "v&<>\"")
            .text("t&<>");
        w.end();
        assert_eq!(
            w.finish(),
            "\n<a k=\"v&amp;&lt;&gt;&quot;\">t&amp;&lt;&gt;</a>"
        );
    }
}
