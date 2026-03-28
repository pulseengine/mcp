//! Core resource router implementation.

use rmcp::model::{Annotated, RawResourceTemplate, ResourceContents, ResourceTemplate};

/// Handler trait for resource handlers.
///
/// Implementors receive the shared state, the original URI, and the extracted
/// matchit params. The handler returns `ResourceContents` for the matched
/// resource.
///
/// A blanket implementation is provided for closures with the signature
/// `Fn(&S, &str, &matchit::Params) -> ResourceContents + Send + Sync`.
pub trait ResourceHandler<S>: Send + Sync {
    /// Handle a matched resource request.
    fn call(&self, state: &S, uri: &str, params: &matchit::Params) -> ResourceContents;
}

impl<S, F> ResourceHandler<S> for F
where
    F: Fn(&S, &str, &matchit::Params) -> ResourceContents + Send + Sync,
{
    fn call(&self, state: &S, uri: &str, params: &matchit::Params) -> ResourceContents {
        (self)(state, uri, params)
    }
}

/// A registered resource route: template metadata, handler, and scheme-strip
/// function for URI-to-route conversion.
struct ResourceRoute<S> {
    template: ResourceTemplate,
    handler: Box<dyn ResourceHandler<S>>,
    /// The URI scheme prefix to strip when resolving concrete URIs.
    /// For example, `"file:///"` for `file:///{path}` templates.
    scheme_prefix: String,
    /// The matchit route prefix that replaces the scheme.
    /// For example, `"/files/"` maps from `file:///` scheme.
    route_prefix: String,
}

/// A URI-template-based resource router built on [`matchit`].
///
/// MCP URI templates use schemes like `file:///` or `config://` that
/// [`matchit`] cannot parse directly. The router maintains a mapping between
/// MCP URI templates and matchit route patterns, handling the conversion
/// transparently.
///
/// The router is generic over state `S` so handlers can access shared server
/// state.
///
/// # Note
///
/// `matchit::Router` does not implement `Debug` or `Clone`, so this type
/// cannot derive those traits either.
pub struct ResourceRouter<S = ()> {
    router: matchit::Router<usize>,
    routes: Vec<ResourceRoute<S>>,
}

impl<S> Default for ResourceRouter<S> {
    fn default() -> Self {
        Self {
            router: matchit::Router::new(),
            routes: Vec::new(),
        }
    }
}

impl<S> ResourceRouter<S> {
    /// Create a new empty resource router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a resource template with a handler.
    ///
    /// # Arguments
    ///
    /// * `route_pattern` — the matchit route pattern (e.g. `"/files/{path}"`)
    /// * `uri_template` — the MCP URI template (e.g. `"file:///{path}"`)
    /// * `name` — human-readable resource name
    /// * `description` — resource description
    /// * `mime_type` — optional MIME type hint
    /// * `handler` — the handler to call when a URI matches this template
    ///
    /// # Panics
    ///
    /// Panics if the route pattern conflicts with an existing route.
    pub fn add_resource(
        &mut self,
        route_pattern: &str,
        uri_template: &str,
        name: &str,
        description: &str,
        mime_type: Option<&str>,
        handler: impl ResourceHandler<S> + 'static,
    ) -> &mut Self {
        let idx = self.routes.len();
        self.router.insert(route_pattern, idx).unwrap_or_else(|e| {
            panic!("Failed to insert route '{route_pattern}' (from '{uri_template}'): {e}")
        });

        // Derive scheme prefix and route prefix from the uri_template and route_pattern.
        // We find the scheme portion of the URI template by locating where the
        // parameterized suffix begins — the part that matches the route_pattern suffix.
        let (scheme_prefix, route_prefix) = derive_prefixes(uri_template, route_pattern);

        let mut raw = RawResourceTemplate::new(uri_template, name).with_description(description);
        if let Some(mime) = mime_type {
            raw = raw.with_mime_type(mime);
        }
        let template = Annotated::new(raw, None);

        self.routes.push(ResourceRoute {
            template,
            handler: Box::new(handler),
            scheme_prefix,
            route_prefix,
        });

        self
    }

    /// Return all registered resource templates.
    ///
    /// Use this in your `ServerHandler::list_resource_templates` implementation.
    pub fn templates(&self) -> Vec<ResourceTemplate> {
        self.routes.iter().map(|r| r.template.clone()).collect()
    }

    /// Match a concrete URI against registered templates and call the handler.
    ///
    /// Returns `None` if no route matches the URI.
    ///
    /// The router converts the URI to a matchit-routable path by applying the
    /// scheme-to-route prefix mappings from registered templates.
    pub fn resolve(&self, state: &S, uri: &str) -> Option<ResourceContents> {
        // Try to convert the URI to a route path using registered scheme mappings
        let route_path = self.uri_to_route_path(uri)?;

        let matched = self.router.at(&route_path).ok()?;
        let route = &self.routes[*matched.value];
        Some(route.handler.call(state, uri, &matched.params))
    }

    /// Convert a concrete URI to a matchit route path using the registered
    /// scheme prefix mappings.
    fn uri_to_route_path(&self, uri: &str) -> Option<String> {
        for route in &self.routes {
            if uri.starts_with(&route.scheme_prefix) {
                let rest = &uri[route.scheme_prefix.len()..];
                return Some(format!("{}{rest}", route.route_prefix));
            }
        }
        None
    }
}

/// Derive the URI scheme prefix and the corresponding matchit route prefix
/// from a URI template and its route pattern.
///
/// For example:
/// - `uri_template = "file:///{path}"`, `route_pattern = "/files/{path}"`
///   yields `("file:///", "/files/")`
/// - `uri_template = "config://{section}/{key}"`, `route_pattern = "/config/{section}/{key}"`
///   yields `("config://", "/config/")`
fn derive_prefixes(uri_template: &str, route_pattern: &str) -> (String, String) {
    // Find where the first `{` appears in both strings — the prefix is everything before.
    let uri_param_start = uri_template.find('{').unwrap_or(uri_template.len());
    let route_param_start = route_pattern.find('{').unwrap_or(route_pattern.len());

    let scheme_prefix = uri_template[..uri_param_start].to_string();
    let route_prefix = route_pattern[..route_param_start].to_string();

    (scheme_prefix, route_prefix)
}

/// Strip the scheme from an MCP URI, returning the path portion.
///
/// This is a convenience function for converting concrete URIs to a form
/// suitable for display or further processing.
///
/// # Examples
///
/// ```
/// use pulseengine_mcp_resources::strip_uri_scheme;
///
/// // file:/// URIs: the third slash is part of the path
/// assert_eq!(strip_uri_scheme("file:///README.md"), "/README.md");
/// assert_eq!(strip_uri_scheme("config://database/host"), "database/host");
/// assert_eq!(strip_uri_scheme("custom://some/path"), "some/path");
/// assert_eq!(strip_uri_scheme("no-scheme"), "no-scheme");
/// ```
pub fn strip_uri_scheme(uri: &str) -> &str {
    uri.find("://").map(|i| &uri[i + 3..]).unwrap_or(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_uri_scheme() {
        // file:/// URIs: the third slash is part of the absolute path
        assert_eq!(strip_uri_scheme("file:///README.md"), "/README.md");
        assert_eq!(strip_uri_scheme("config://database/host"), "database/host");
        assert_eq!(strip_uri_scheme("custom://some/path"), "some/path");
        assert_eq!(strip_uri_scheme("no-scheme"), "no-scheme");
    }

    #[test]
    fn test_derive_prefixes() {
        let (scheme, route) = derive_prefixes("file:///{path}", "/files/{path}");
        assert_eq!(scheme, "file:///");
        assert_eq!(route, "/files/");

        let (scheme, route) =
            derive_prefixes("config://{section}/{key}", "/config/{section}/{key}");
        assert_eq!(scheme, "config://");
        assert_eq!(route, "/config/");
    }
}
