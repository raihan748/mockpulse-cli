//! Radix Trie path router with path parameter extraction and wildcard support.

use super::route::{HttpMethod, RouteEndpoint};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentType {
    Exact(String),
    Param(String), // {id} or :id
    Wildcard(String), // *tail
}

#[derive(Debug, Default)]
pub struct TrieNode {
    pub segment: Option<SegmentType>,
    pub endpoints: HashMap<HttpMethod, RouteEndpoint>,
    pub children: Vec<TrieNode>,
}

impl TrieNode {
    pub fn new(segment: Option<SegmentType>) -> Self {
        Self {
            segment,
            endpoints: HashMap::new(),
            children: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct RouteMatch<'a> {
    pub endpoint: &'a RouteEndpoint,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct RadixTrieRouter {
    root: TrieNode,
    routes_count: usize,
}

impl RadixTrieRouter {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(None),
            routes_count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.routes_count
    }

    pub fn is_empty(&self) -> bool {
        self.routes_count == 0
    }

    pub fn parse_segment(raw: &str) -> SegmentType {
        let trimmed = raw.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            let name = &trimmed[1..trimmed.len() - 1];
            SegmentType::Param(name.to_string())
        } else if trimmed.starts_with(':') {
            let name = &trimmed[1..];
            SegmentType::Param(name.to_string())
        } else if trimmed.starts_with('*') {
            let name = &trimmed[1..];
            SegmentType::Wildcard(if name.is_empty() { "wildcard".to_string() } else { name.to_string() })
        } else {
            SegmentType::Exact(trimmed.to_string())
        }
    }

    pub fn insert(&mut self, endpoint: RouteEndpoint) {
        let clean_path = endpoint.path_pattern.trim();
        let segments: Vec<&str> = clean_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut current = &mut self.root;

        for seg_str in segments {
            let seg_type = Self::parse_segment(seg_str);

            let idx = match current.children.iter().position(|c| c.segment.as_ref() == Some(&seg_type)) {
                Some(i) => i,
                None => {
                    current.children.push(TrieNode::new(Some(seg_type)));
                    current.children.len() - 1
                }
            };

            current = &mut current.children[idx];
        }

        current.endpoints.insert(endpoint.method, endpoint);
        self.routes_count += 1;
    }

    pub fn lookup<'a>(&'a self, method: &HttpMethod, path: &str) -> Option<RouteMatch<'a>> {
        let clean_path = path.trim();
        let segments: Vec<&str> = clean_path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut params = HashMap::new();
        if let Some(endpoint) = Self::search_node(&self.root, &segments, 0, method, &mut params) {
            Some(RouteMatch { endpoint, params })
        } else {
            None
        }
    }

    fn search_node<'a>(
        node: &'a TrieNode,
        segments: &[&str],
        depth: usize,
        method: &HttpMethod,
        params: &mut HashMap<String, String>,
    ) -> Option<&'a RouteEndpoint> {
        if depth == segments.len() {
            return node.endpoints.get(method);
        }

        let current_seg = segments[depth];

        // 1. Try exact match first
        for child in &node.children {
            if let Some(SegmentType::Exact(ref exact)) = child.segment {
                if exact.eq_ignore_ascii_case(current_seg) {
                    if let Some(matched) = Self::search_node(child, segments, depth + 1, method, params) {
                        return Some(matched);
                    }
                }
            }
        }

        // 2. Try parameterized match (:id or {id})
        for child in &node.children {
            if let Some(SegmentType::Param(ref param_name)) = child.segment {
                params.insert(param_name.clone(), current_seg.to_string());
                if let Some(matched) = Self::search_node(child, segments, depth + 1, method, params) {
                    return Some(matched);
                }
                params.remove(param_name);
            }
        }

        // 3. Try wildcard match (*rest)
        for child in &node.children {
            if let Some(SegmentType::Wildcard(ref wildcard_name)) = child.segment {
                let tail = segments[depth..].join("/");
                params.insert(wildcard_name.clone(), tail);
                if let Some(ep) = child.endpoints.get(method) {
                    return Some(ep);
                }
                params.remove(wildcard_name);
            }
        }

        None
    }

    /// Returns all registered routes in the trie.
    pub fn all_routes(&self) -> Vec<&RouteEndpoint> {
        let mut routes = Vec::new();
        Self::collect_routes(&self.root, &mut routes);
        routes
    }

    fn collect_routes<'a>(node: &'a TrieNode, out: &mut Vec<&'a RouteEndpoint>) {
        for ep in node.endpoints.values() {
            out.push(ep);
        }
        for child in &node.children {
            Self::collect_routes(child, out);
        }
    }
}
