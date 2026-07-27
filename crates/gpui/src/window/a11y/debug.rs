//! Developer tooling for inspecting the latest accessibility tree.

use accesskit::{Node, NodeId, TreeUpdate};
use collections::FxHashMap;
use serde_json::{Map, Value, json};

pub(crate) fn tree_update_to_json(update: &TreeUpdate) -> String {
    let mut nodes = update.nodes.iter().collect::<Vec<_>>();
    nodes.sort_by_key(|(id, _)| id.0);

    let debug_ids = nodes
        .iter()
        .enumerate()
        .map(|(index, (id, _))| (*id, debug_id(index)))
        .collect::<FxHashMap<_, _>>();
    let nodes = nodes
        .into_iter()
        .map(|(id, node)| node_to_json(*id, node, &debug_ids))
        .collect::<Vec<_>>();
    let root = update
        .tree
        .as_ref()
        .map(|tree| tree.root)
        .unwrap_or(crate::window::a11y::ROOT_NODE_ID);
    let value = json!({
        "root": id_for(root, &debug_ids),
        "focus": id_for(update.focus, &debug_ids),
        "nodes": nodes,
    });

    serde_json::to_string_pretty(&value).unwrap_or_else(|error| {
        log::error!("failed to serialize accessibility debug tree: {error}");
        "{}".to_string()
    })
}

fn node_to_json(id: NodeId, node: &Node, debug_ids: &FxHashMap<NodeId, String>) -> Value {
    let mut node_json = Map::new();
    node_json.insert("id".into(), json!(id_for(id, debug_ids)));
    node_json.insert("accesskit_id".into(), json!(id.0.to_string()));

    if !node.children().is_empty() {
        node_json.insert(
            "children".into(),
            json!(
                node.children()
                    .iter()
                    .map(|child| id_for(*child, debug_ids))
                    .collect::<Vec<_>>()
            ),
        );
    }

    node_json.insert("aria".into(), Value::Object(aria_to_json(node)));
    Value::Object(node_json)
}

fn aria_to_json(node: &Node) -> Map<String, Value> {
    let mut aria = Map::new();
    aria.insert("role".into(), json!(format!("{:?}", node.role())));

    insert_string(&mut aria, "label", node.label());
    insert_string(&mut aria, "description", node.description());
    insert_string(&mut aria, "keyboard_shortcut", node.keyboard_shortcut());
    insert_string(&mut aria, "value", node.value());
    insert_string(&mut aria, "placeholder", node.placeholder());
    insert_string(&mut aria, "tooltip", node.tooltip());
    insert_string(&mut aria, "role_description", node.role_description());

    if node.is_disabled() {
        aria.insert("disabled".into(), json!(true));
    }
    if node.is_required() {
        aria.insert("required".into(), json!(true));
    }
    if let Some(value) = node.invalid() {
        aria.insert("invalid".into(), json!(format!("{value:?}")));
    }
    if node.is_modal() {
        aria.insert("modal".into(), json!(true));
    }
    if let Some(value) = node.is_selected() {
        aria.insert("selected".into(), json!(value));
    }
    if let Some(value) = node.is_expanded() {
        aria.insert("expanded".into(), json!(value));
    }
    if let Some(value) = node.toggled() {
        aria.insert("toggled".into(), json!(format!("{value:?}")));
    }
    if let Some(value) = node.orientation() {
        aria.insert("orientation".into(), json!(format!("{value:?}")));
    }

    insert_number(&mut aria, "numeric_value", node.numeric_value());
    insert_number(&mut aria, "min_numeric_value", node.min_numeric_value());
    insert_number(&mut aria, "max_numeric_value", node.max_numeric_value());
    insert_number(&mut aria, "numeric_value_step", node.numeric_value_step());
    insert_usize(&mut aria, "level", node.level());
    insert_usize(&mut aria, "position_in_set", node.position_in_set());
    insert_usize(&mut aria, "size_of_set", node.size_of_set());
    insert_usize(&mut aria, "row_index", node.row_index());
    insert_usize(&mut aria, "column_index", node.column_index());
    insert_usize(&mut aria, "row_count", node.row_count());
    insert_usize(&mut aria, "column_count", node.column_count());

    aria
}

fn id_for(id: NodeId, debug_ids: &FxHashMap<NodeId, String>) -> String {
    debug_ids
        .get(&id)
        .cloned()
        .unwrap_or_else(|| id.0.to_string())
}

/// Maps a zero-based index to `a, b, ..., z, aa, ab, ...`.
fn debug_id(mut index: usize) -> String {
    let mut bytes = Vec::new();
    loop {
        bytes.push(b'a' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    bytes.reverse();
    String::from_utf8(bytes).unwrap_or_default()
}

fn insert_string(target: &mut Map<String, Value>, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        target.insert(name.into(), json!(value));
    }
}

fn insert_number(target: &mut Map<String, Value>, name: &str, value: Option<f64>) {
    if let Some(value) = value {
        target.insert(name.into(), json!(value));
    }
}

fn insert_usize(target: &mut Map<String, Value>, name: &str, value: Option<usize>) {
    if let Some(value) = value {
        target.insert(name.into(), json!(value));
    }
}

#[cfg(test)]
mod tests {
    use super::debug_id;

    #[test]
    fn debug_ids_remain_short_after_z() {
        assert_eq!(debug_id(0), "a");
        assert_eq!(debug_id(25), "z");
        assert_eq!(debug_id(26), "aa");
        assert_eq!(debug_id(27), "ab");
    }
}
