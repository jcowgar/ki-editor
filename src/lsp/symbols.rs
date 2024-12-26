use crate::{
    app::{Dispatch, Dispatches},
    components::dropdown::DropdownItem,
    quickfix_list::Location,
};
use lsp_types::{DocumentSymbolResponse, SymbolKind};
use shared::{canonicalized_path::CanonicalizedPath, icons::get_icon_config};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbols {
    pub(crate) symbols: Vec<Symbol>,
}

impl Symbols {
    fn collect_document_symbols(
        document_symbol: &lsp_types::DocumentSymbol,
        parent_name: Option<String>,
        path: &CanonicalizedPath,
    ) -> Result<Vec<Symbol>, anyhow::Error> {
        Ok(std::iter::once(Symbol::try_from_document_symbol(
            document_symbol.clone(),
            path.clone(),
            &parent_name,
        )?)
        .chain(
            document_symbol
                .children
                .iter()
                .flatten()
                .map(|child| {
                    let parent_name = format!(
                        "{}{}",
                        parent_name
                            .as_ref()
                            .map(|name| format!("{name} ▶ ",))
                            .unwrap_or_default(),
                        document_symbol.name.clone()
                    );
                    Self::collect_document_symbols(child, Some(parent_name), path)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten(),
        )
        .collect())
    }

    pub(crate) fn try_from_document_symbol_response(
        value: DocumentSymbolResponse,
        path: CanonicalizedPath,
    ) -> anyhow::Result<Self> {
        match value {
            DocumentSymbolResponse::Flat(symbols) => {
                let symbols = symbols
                    .into_iter()
                    .map(|symbol| symbol.try_into())
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self { symbols })
            }
            DocumentSymbolResponse::Nested(symbols) => Ok(Self {
                symbols: symbols
                    .iter()
                    .map(|symbol| Self::collect_document_symbols(symbol, None, &path))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect(),
            }),
        }
    }
}

impl TryFrom<lsp_types::SymbolInformation> for Symbol {
    type Error = anyhow::Error;

    fn try_from(value: lsp_types::SymbolInformation) -> Result<Self, Self::Error> {
        let name = value.name;
        let location = value.location.try_into()?;
        Ok(Self {
            name,
            kind: value.kind,
            location,
            container_name: value.container_name,
        })
    }
}

impl Symbol {
    fn try_from_document_symbol(
        value: lsp_types::DocumentSymbol,
        path: CanonicalizedPath,
        container_name: &Option<String>,
    ) -> anyhow::Result<Self> {
        let name = value.name;
        let start_position = value.range.start.into();
        let end_position = value.range.end.into();
        Ok(Self {
            name,
            kind: value.kind,
            location: Location {
                path,
                range: start_position..end_position,
            },
            container_name: container_name.as_ref().cloned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbol {
    pub(crate) name: String,
    pub(crate) kind: SymbolKind,
    pub(crate) location: Location,
    pub(crate) container_name: Option<String>,
}
impl Symbol {
    pub(crate) fn display(&self) -> String {
        let icon = get_icon_config()
            .completion
            .get(&format!("{:?}", self.kind))
            .cloned()
            .unwrap_or_default();
        format!("{} {}", icon, self.name)
    }
}

impl From<Symbol> for DropdownItem {
    fn from(symbol: Symbol) -> Self {
        let dispatches = Dispatches::one(Dispatch::GotoLocation(symbol.location.to_owned()));
        DropdownItem::new(symbol.display())
            .set_group(Some(
                symbol.container_name.unwrap_or("[TOP LEVEL]".to_string()),
            ))
            .set_dispatches(dispatches)
    }
}
