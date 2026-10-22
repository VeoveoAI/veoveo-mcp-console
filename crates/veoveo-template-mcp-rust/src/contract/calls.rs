//! Typed calls and resource associations, available without the server runtime.
use super::{ItemId, ItemView, TemplateResource};
use veoveo_hub_sdk::calls::ProviderContract;
use veoveo_types::{Collection, LiveKeyset, PageSize, ServerSlug};

pub struct TemplateProvider;
impl ProviderContract for TemplateProvider {
    fn slug() -> ServerSlug {
        crate::server()
    }
}

/// Tenant items in ascending UUIDv7 creation order, with unique identity tiebreaker.
pub struct Items;
impl Collection for Items {
    type Item = ItemView;
    type Filters = veoveo_hub_sdk::definition::NoFilters;
    type Key = ItemId;
    type Limit = PageSize<100>;
    type Policy = LiveKeyset;
    const NAME: &'static str = "template.items";
    fn key(item: &ItemView) -> ItemId {
        item.item_id
    }
}

#[derive(Debug, Clone)]
#[veoveo_hub_sdk::collection_resource(provider = TemplateProvider, address_type = TemplateResource,
    collection = Items, template = "template://items", address = TemplateResource::Items,
    filters = veoveo_hub_sdk::definition::NoFilters {},
    parse = |address| matches!(address, TemplateResource::Items).then_some(Self))]
pub struct ReadItems;

#[derive(Debug, Clone)]
#[veoveo_hub_sdk::resource_read(provider = TemplateProvider, address_type = TemplateResource,
    content = ItemView, template = "template://item/{item_id}", address = TemplateResource::Item(self.item),
    parse = |address| match address {TemplateResource::Item(item) => Some(Self {item:*item}), _=>None})]
pub struct ReadItem {
    pub item: ItemId,
}
