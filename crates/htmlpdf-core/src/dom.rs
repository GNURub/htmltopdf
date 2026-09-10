use std::collections::BTreeMap;

pub type NodeId = usize;

#[derive(Debug, Clone)]
pub struct Document {
    nodes: Vec<Node>,
    parents: Vec<Option<NodeId>>,
    root: NodeId,
}

impl Document {
    pub fn new() -> Self {
        let root = Node::Element(ElementNode {
            tag: "document".to_string(),
            attrs: BTreeMap::new(),
            children: Vec::new(),
        });
        Self {
            nodes: vec![root],
            parents: vec![None],
            root: 0,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub fn push_text(&mut self, parent: NodeId, text: impl Into<String>) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node::Text(text.into()));
        self.parents.push(None);
        self.append_child(parent, id);
        id
    }

    pub fn push_element(
        &mut self,
        parent: NodeId,
        tag: impl Into<String>,
        attrs: BTreeMap<String, String>,
    ) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node::Element(ElementNode {
            tag: tag.into().to_ascii_lowercase(),
            attrs,
            children: Vec::new(),
        }));
        self.parents.push(None);
        self.append_child(parent, id);
        id
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        if let Some(Node::Element(element)) = self.nodes.get_mut(parent) {
            element.children.push(child);
            if let Some(slot) = self.parents.get_mut(child) {
                *slot = Some(parent);
            }
        }
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match self.node(id) {
            Some(Node::Element(element)) => &element.children,
            _ => &[],
        }
    }

    pub fn parent_of(&self, child: NodeId) -> Option<NodeId> {
        self.parents.get(child).copied().flatten()
    }

    pub fn remove_children(&mut self, id: NodeId) {
        let children = match self.node(id) {
            Some(Node::Element(element)) => element.children.clone(),
            _ => Vec::new(),
        };
        for child in children {
            if let Some(slot) = self.parents.get_mut(child) {
                *slot = None;
            }
        }
        if let Some(Node::Element(element)) = self.node_mut(id) {
            element.children.clear();
        }
    }

    pub fn set_text_content(&mut self, id: NodeId, text: impl Into<String>) {
        self.remove_children(id);
        self.push_text(id, text.into());
    }

    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.collect_text(id, &mut out);
        out
    }

    pub fn scripts(&self) -> Vec<String> {
        let mut scripts = Vec::new();
        self.collect_scripts(self.root, &mut scripts);
        scripts
    }

    pub fn styles(&self) -> Vec<String> {
        let mut styles = Vec::new();
        self.collect_styles(self.root, &mut styles);
        styles
    }

    pub fn style_sources(&self) -> Vec<StyleSource> {
        let mut sources = Vec::new();
        self.collect_style_sources(self.root, &mut sources);
        sources
    }

    pub fn query_selector(&self, selector: &str) -> Option<NodeId> {
        let parsed = SimpleSelector::parse(selector)?;
        self.find_matching(self.root, &parsed)
    }

    pub fn traverse_preorder(&self) -> Vec<NodeId> {
        let mut ids = Vec::new();
        self.walk(self.root, &mut ids);
        ids
    }

    fn append_text_with_space(out: &mut String, text: &str) {
        let normalized = text
            .split(|character| matches!(character, ' ' | '\t' | '\r' | '\n' | '\u{000c}'))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if normalized.is_empty() {
            return;
        }
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(&normalized);
    }

    fn append_hard_break(out: &mut String) {
        while out.ends_with(' ') {
            out.pop();
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
    }

    fn collect_text(&self, id: NodeId, out: &mut String) {
        match self.node(id) {
            Some(Node::Text(text)) => Self::append_text_with_space(out, text),
            Some(Node::Element(element)) => {
                if element.tag == "script" || element.tag == "style" {
                    return;
                }
                if element.tag == "br" {
                    Self::append_hard_break(out);
                    return;
                }
                for child in &element.children {
                    self.collect_text(*child, out);
                }
            }
            None => {}
        }
    }

    fn collect_scripts(&self, id: NodeId, scripts: &mut Vec<String>) {
        if let Some(Node::Element(element)) = self.node(id) {
            if element.tag == "script" {
                scripts.push(self.raw_text(id));
                return;
            }
            for child in &element.children {
                self.collect_scripts(*child, scripts);
            }
        }
    }

    fn collect_styles(&self, id: NodeId, styles: &mut Vec<String>) {
        if let Some(Node::Element(element)) = self.node(id) {
            if element.tag == "style" {
                styles.push(self.raw_text(id));
                return;
            }
            for child in &element.children {
                self.collect_styles(*child, styles);
            }
        }
    }

    fn collect_style_sources(&self, id: NodeId, sources: &mut Vec<StyleSource>) {
        if let Some(Node::Element(element)) = self.node(id) {
            if element.tag == "noscript" {
                return;
            }
            if element.tag == "style" {
                sources.push(StyleSource::Inline(self.raw_text(id)));
                return;
            }
            if element.tag == "link"
                && element
                    .attr("rel")
                    .is_some_and(|rel| rel.split_whitespace().any(|token| token == "stylesheet"))
            {
                if let Some(href) = element.attr("href") {
                    sources.push(StyleSource::Linked {
                        href: href.to_string(),
                        media: element.attr("media").map(str::to_string),
                    });
                }
            }
            for child in &element.children {
                self.collect_style_sources(*child, sources);
            }
        }
    }

    fn raw_text(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.collect_raw_text(id, &mut out);
        out
    }

    fn collect_raw_text(&self, id: NodeId, out: &mut String) {
        match self.node(id) {
            Some(Node::Text(text)) => out.push_str(text),
            Some(Node::Element(element)) => {
                for child in &element.children {
                    self.collect_raw_text(*child, out);
                }
            }
            None => {}
        }
    }

    fn find_matching(&self, id: NodeId, selector: &SimpleSelector) -> Option<NodeId> {
        if let Some(Node::Element(element)) = self.node(id) {
            if selector.matches(element) {
                return Some(id);
            }
            for child in &element.children {
                if let Some(found) = self.find_matching(*child, selector) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn walk(&self, id: NodeId, ids: &mut Vec<NodeId>) {
        ids.push(id);
        if let Some(Node::Element(element)) = self.node(id) {
            for child in &element.children {
                self.walk(*child, ids);
            }
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Element(ElementNode),
    Text(String),
}

#[derive(Debug, Clone)]
pub enum StyleSource {
    Inline(String),
    Linked { href: String, media: Option<String> },
}

#[derive(Debug, Clone)]
pub struct ElementNode {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<NodeId>,
}

impl ElementNode {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.attr("class")
            .into_iter()
            .flat_map(|value| value.split_whitespace())
    }
}

#[derive(Debug)]
struct SimpleSelector<'a> {
    raw: &'a str,
}

impl<'a> SimpleSelector<'a> {
    fn parse(raw: &'a str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self { raw: trimmed })
        }
    }

    fn matches(&self, element: &ElementNode) -> bool {
        if let Some(id) = self.raw.strip_prefix('#') {
            return element.attr("id") == Some(id);
        }
        if let Some(class) = self.raw.strip_prefix('.') {
            return element.classes().any(|candidate| candidate == class);
        }
        element.tag == self.raw.to_ascii_lowercase()
    }
}
