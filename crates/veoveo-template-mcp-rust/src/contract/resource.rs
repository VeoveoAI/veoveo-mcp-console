//! The server's resource addresses under the `template` scheme. The route declaration
//! owns each shape and its typed parameters; the shared URI machinery owns encoding and
//! parsing, so no address is built or split by hand.
use std::{borrow::Cow, error::Error, fmt};

use veoveo_types::{ResourceFieldCodec, ResourceRouteError, ResourceUriError, Vocabulary};

use super::ItemId;

/// A resource address that failed admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateResourceError {
    Uri(ResourceUriError),
    UnknownResource,
    InvalidId,
}

impl fmt::Display for TemplateResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uri(error) => error.fmt(f),
            Self::UnknownResource => {
                f.write_str("unknown or noncanonical template resource address")
            }
            Self::InvalidId => f.write_str("invalid item identity"),
        }
    }
}

impl Error for TemplateResourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Uri(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceUriError> for TemplateResourceError {
    fn from(error: ResourceUriError) -> Self {
        Self::Uri(error)
    }
}

/// A document the server embeds and serves under `template://docs/{doc_id}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Vocabulary)]
pub enum TemplateDocument {
    /// The agent work manual, `AGENTS.md`.
    Agents,
    /// The design, `DESIGN.md`.
    Design,
}

/// A resource of the server. A usable address carries only the identities and cursor of
/// its own family.
#[veoveo_types::resource_address(routes(TemplateAddresses), schema = derived, wire, parse = borrowed, display)]
pub enum TemplateResource {
    #[resource(template = "template://docs")]
    Docs,
    #[resource(template = "template://docs/{doc_id}")]
    Document(#[resource(variable = "doc_id", codec = TemplateFieldCodec)] TemplateDocument),
    #[resource(template = "template://contract")]
    Contract,
    #[resource(template = "template://items")]
    Items,
    #[resource(template = "template://item/{item_id}")]
    Item(#[resource(variable = "item_id", codec = TemplateFieldCodec)] ItemId),
}

impl TemplateResource {
    /// The first page of the caller's items.
    pub fn items() -> Self {
        Self::Items
    }
}

fn route_error(error: ResourceRouteError) -> TemplateResourceError {
    match error {
        ResourceRouteError::Uri(error) => TemplateResourceError::Uri(error),
        _ => TemplateResourceError::UnknownResource,
    }
}

struct TemplateFieldCodec;

impl ResourceFieldCodec<TemplateDocument> for TemplateFieldCodec {
    type Error = TemplateResourceError;
    fn parse(value: &str) -> Result<TemplateDocument, Self::Error> {
        value
            .parse()
            .map_err(|_| TemplateResourceError::UnknownResource)
    }
    fn text(value: &TemplateDocument) -> Cow<'_, str> {
        value.as_str().into()
    }
}

impl ResourceFieldCodec<ItemId> for TemplateFieldCodec {
    type Error = TemplateResourceError;
    fn parse(value: &str) -> Result<ItemId, Self::Error> {
        ItemId::parse(value).map_err(|_| TemplateResourceError::InvalidId)
    }
    fn text(value: &ItemId) -> Cow<'_, str> {
        value.to_string().into()
    }
}

#[doc(hidden)]
pub struct TemplateAddresses;
impl veoveo_types::ResourceProfile for TemplateAddresses {
    type Error = TemplateResourceError;
    const PROFILE: veoveo_types::ResourceProfileSpec<Self::Error> =
        veoveo_types::ResourceProfileSpec {
            route_error: |_, error| route_error(error),
        };
}

#[cfg(test)]
mod tests {
    use super::*;
    use veoveo_types::ResourceAddress;

    const ITEM: &str = "0190f3a2-7b1c-7d2e-8f3a-4b5c6d7e8f90";

    fn parse(uri: &str) -> Result<TemplateResource, TemplateResourceError> {
        TemplateResource::parse(uri)
    }

    #[test]
    fn addresses_round_trip_through_their_routes() {
        for uri in [
            "template://docs".to_owned(),
            "template://docs/agents".to_owned(),
            "template://contract".to_owned(),
            "template://items".to_owned(),
            format!("template://item/{ITEM}"),
        ] {
            let address = parse(&uri).unwrap_or_else(|error| panic!("{uri}: {error}"));
            assert_eq!(address.to_uri().unwrap().as_str(), uri);
        }
        assert_eq!(
            parse(&format!("template://item/{ITEM}")).unwrap(),
            TemplateResource::Item(ItemId::parse(ITEM).unwrap())
        );
    }

    #[test]
    fn malformed_addresses_are_refused() {
        for uri in [
            "template://item".to_owned(),
            "template://item/".to_owned(),
            "template://item/not-an-item".to_owned(),
            format!("template://item/{ITEM}/extra"),
            "template://items?cursor=a=b".to_owned(),
            "template://items?unknown=1".to_owned(),
            "template://docs/unknown".to_owned(),
            "template://other".to_owned(),
            "media://items".to_owned(),
        ] {
            assert!(parse(&uri).is_err(), "{uri}");
        }
    }
}
