use super::*;
use crate::ir::object::SymbolBinding;

pub(super) struct Symbols {
    pub exports: BTreeMap<String, u32>,
    locals: Vec<BTreeMap<String, u32>>,
}
impl Symbols {
    pub fn build(
        objects: &[Object],
        offsets: &BTreeMap<(usize, SectionKind), u32>,
        sections: &[LinkedSection],
        plan: &LinkPlan,
        heap: u32,
        stack: u32,
    ) -> Result<Self, LinkError> {
        if plan.entry_symbol.is_empty()
            || plan.heap_base_symbol.is_empty()
            || plan.stack_bottom_symbol.is_empty()
            || plan.heap_base_symbol == plan.stack_bottom_symbol
            || plan.entry_symbol == plan.heap_base_symbol
            || plan.entry_symbol == plan.stack_bottom_symbol
        {
            return Err(link_error("invalid plan symbol names"));
        }
        let mut result = Self {
            exports: BTreeMap::from([
                (plan.heap_base_symbol.clone(), heap),
                (plan.stack_bottom_symbol.clone(), stack),
            ]),
            locals: vec![BTreeMap::new(); objects.len()],
        };
        for (index, o) in objects.iter().enumerate() {
            for s in &o.symbols {
                if s.binding == SymbolBinding::Imported {
                    continue;
                }
                if s.name == plan.heap_base_symbol || s.name == plan.stack_bottom_symbol {
                    return Err(link_error(format!("reserved symbol collision: {}", s.name)));
                }
                let k = s
                    .section
                    .ok_or_else(|| link_error("missing symbol section"))?;
                let section = sections
                    .iter()
                    .find(|x| x.kind == k)
                    .ok_or_else(|| link_error("missing linked section"))?;
                let address = section
                    .load_address
                    .checked_add(offsets[&(index, k)])
                    .and_then(|a| a.checked_add(s.offset))
                    .ok_or_else(|| link_error("symbol address overflow"))?;
                let map = if s.binding == SymbolBinding::Local {
                    &mut result.locals[index]
                } else {
                    &mut result.exports
                };
                if map.insert(s.name.clone(), address).is_some() {
                    return Err(link_error(format!("duplicate export: {}", s.name)));
                }
            }
        }
        for (i, o) in objects.iter().enumerate() {
            for s in o
                .symbols
                .iter()
                .filter(|s| s.binding == SymbolBinding::Imported)
            {
                result.resolve(i, &s.name)?;
            }
        }
        Ok(result)
    }
    pub fn resolve(&self, object: usize, name: &str) -> Result<u32, LinkError> {
        self.locals[object]
            .get(name)
            .or_else(|| self.exports.get(name))
            .copied()
            .ok_or_else(|| link_error(format!("unknown relocation target `{name}`")))
    }
}
