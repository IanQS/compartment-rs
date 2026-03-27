use std::collections::HashMap;

use crate::swc_reader::Node;

#[derive(Debug, Clone)]
pub(crate) struct Section {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub children_ids: Vec<u64>,
    pub length: f64,
    pub mean_diam: f64,
    pub swc_nodes: Vec<u64>,
}

fn filtered_children(parent_child_map: &HashMap<u64, Vec<u64>>, node_id: u64) -> Vec<u64> {
    parent_child_map
        .get(&node_id)
        .map(|children| {
            children
                .iter()
                .copied()
                .filter(|&child| child != node_id)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn coalesce_into_sections(
    sorted_nodes: &[Node],
    parent_child_map: &HashMap<u64, Vec<u64>>,
) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut starter_nodes = Vec::new();

    // Helper: check if a node is the soma root (self-referencing parent)
    let is_soma_root = |n: &Node| n.parent_id == n.node_id;

    // Helper: check if a node has soma-type structure identifier
    let is_soma_type =
        |n: &Node| n.structured_identifier == crate::swc_reader::StructureIdentifier::Soma;

    // 1. Identify starter nodes
    for node in sorted_nodes {
        if is_soma_root(node) {
            starter_nodes.push(node.node_id);
            continue;
        }

        let parent_id = node.parent_id;
        let parent_node = sorted_nodes
            .iter()
            .find(|n| n.node_id == parent_id)
            .expect("parent must exist in sorted nodes");

        // A soma-type child of the soma root is NOT a starter — it will be
        // absorbed into the soma section during tracing.
        if is_soma_root(parent_node) && is_soma_type(node) {
            continue;
        }

        let is_parent_soma = is_soma_root(parent_node);
        let parent_children = filtered_children(parent_child_map, parent_id).len();

        if is_parent_soma || parent_children > 1 {
            starter_nodes.push(node.node_id);
        }
    }

    let node_map: HashMap<u64, &Node> = sorted_nodes.iter().map(|n| (n.node_id, n)).collect();
    let mut node_to_section = HashMap::new();

    // 2. Trace sections
    for (sec_idx, &starter_id) in starter_nodes.iter().enumerate() {
        let sec_id = sec_idx as u64;
        let mut curr_id = starter_id;
        let mut swc_nodes = vec![curr_id];
        let mut length = 0.0;
        let mut sum_diam = node_map[&curr_id].radius * 2.0;

        let starter_is_soma_root = is_soma_root(node_map[&starter_id]);
        if !starter_is_soma_root {
            let parent_id = node_map[&curr_id].parent_id;
            length += compute_length(node_map[&curr_id], node_map[&parent_id]);
        }

        loop {
            node_to_section.insert(curr_id, sec_id);
            let children = filtered_children(parent_child_map, curr_id);

            if children.is_empty() || children.len() > 1 {
                // For the soma root, check if exactly one child is soma-type;
                // if so, trace into that child to build a multi-node soma section.
                if starter_is_soma_root && !children.is_empty() {
                    let soma_children: Vec<u64> = children
                        .iter()
                        .copied()
                        .filter(|&cid| is_soma_type(node_map[&cid]))
                        .collect();
                    if soma_children.len() == 1 {
                        let child_id = soma_children[0];
                        length +=
                            compute_length(node_map[&curr_id], node_map[&child_id]);
                        sum_diam += node_map[&child_id].radius * 2.0;
                        swc_nodes.push(child_id);
                        curr_id = child_id;
                        continue;
                    }
                }
                break;
            }

            let child_id = children[0];
            length += compute_length(node_map[&curr_id], node_map[&child_id]);
            sum_diam += node_map[&child_id].radius * 2.0;
            swc_nodes.push(child_id);
            curr_id = child_id;
        }

        sections.push(Section {
            id: sec_id,
            parent_id: None,
            children_ids: vec![],
            length,
            mean_diam: sum_diam / (swc_nodes.len() as f64),
            swc_nodes,
        });
    }

    // 3. Wire topology
    for section in &mut sections {
        let first_node = node_map[&section.swc_nodes[0]];
        if first_node.parent_id != first_node.node_id {
            if let Some(&parent_sec_id) = node_to_section.get(&first_node.parent_id) {
                section.parent_id = Some(parent_sec_id);
            }
        }
    }

    let parent_child_edges: Vec<(u64, u64)> = sections
        .iter()
        .filter_map(|section| section.parent_id.map(|pid| (pid, section.id)))
        .collect();

    for (parent_id, child_id) in parent_child_edges {
        if let Some(parent_section) = sections.get_mut(parent_id as usize) {
            parent_section.children_ids.push(child_id);
        }
    }

    sections
}

/// Assumes simple direct path between the nodes
fn compute_length(curr: &Node, other: &Node) -> f64 {
    let x_diff = (curr.x_pos - other.x_pos).powi(2);
    let y_diff = (curr.y_pos - other.y_pos).powi(2);
    let z_diff = (curr.z_pos - other.z_pos).powi(2);
    (x_diff + y_diff + z_diff).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swc_reader::swc_reader;

    #[test]
    fn coalesce_basic_morphology_sections_and_topology() {
        let (nodes, parent_child_map, _) =
            swc_reader("data/basic.swc".to_string(), Some(true), Some(false), None).unwrap();

        let sections = coalesce_into_sections(&nodes, &parent_child_map);
        assert_eq!(sections.len(), 4);

        let root = sections.iter().find(|s| s.parent_id.is_none()).unwrap();
        assert_eq!(root.children_ids.len(), 3);
    }
}
