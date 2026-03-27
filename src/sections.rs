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

/// Groups topologically-sorted SWC nodes into cable **sections** — contiguous,
/// unbranched stretches of neurite that will each become one (or more, after the
/// d_lambda rule) compartments in the simulation.
///
/// A section spans from a branch-start node to the node just before the next fork
/// or leaf, capturing the total Euclidean length and mean diameter of that stretch.
///
/// # Algorithm
///
/// The function runs in three phases:
///
/// **Phase 1 — Soma:** All consecutive soma-type nodes (SWC type 1) starting from
/// the root are coalesced into a single soma section (always section id 0).
///
/// **Phase 2 — Dendrites / Axons:** Every non-soma node that is either (a) a direct
/// child of a soma node or (b) a child of a branching node (parent has >1 non-soma
/// children) becomes a *starter* node for a new section. Each section is then traced
/// forward through single-child nodes until a leaf or fork is reached.
///
/// **Phase 3 — Topology wiring:** `parent_id` and `children_ids` are filled in for
/// every section by consulting the `node_to_section` map built during phases 1 and 2.
pub(crate) fn coalesce_into_sections(
    sorted_nodes: &[Node],
    parent_child_map: &HashMap<u64, Vec<u64>>,
) -> Vec<Section> {
    use crate::swc_reader::StructureIdentifier;

    let node_map: HashMap<u64, &Node> = sorted_nodes.iter().map(|n| (n.node_id, n)).collect();
    let mut node_to_section: HashMap<u64, u64> = HashMap::new();
    let mut sections: Vec<Section> = Vec::new();

    let is_soma_type = |n: &Node| n.structured_identifier == StructureIdentifier::Soma;

    // ----------------------------------------------------------------
    // Phase 1:
    // Build up the soma sections into a single soma, which reflects what Jaxley does
    // In the SWC, there can be many entries with the type of Soma, but only ever one
    // that has the parent ID of -1
    // ----------------------------------------------------------------
    let soma_root = sorted_nodes
        .iter()
        .find(|n| n.parent_id == n.node_id)
        .expect("SWC must have exactly one root node");

    let mut soma_nodes: Vec<u64> = vec![soma_root.node_id];
    let mut soma_length = 0.0_f64;
    let mut soma_sum_diam = soma_root.radius * 2.0;
    let mut curr_id = soma_root.node_id;

    loop {
        node_to_section.insert(curr_id, 0);
        let soma_child: Vec<u64> = filtered_children(parent_child_map, curr_id)
            .into_iter()
            .filter(|&cid| is_soma_type(node_map[&cid]))
            .collect();
        // Continue only if there is exactly one unambiguous soma-type child.
        if soma_child.len() != 1 {
            break;
        }
        let child_id = soma_child[0];
        soma_length += compute_length(node_map[&curr_id], node_map[&child_id]);
        soma_sum_diam += node_map[&child_id].radius * 2.0;
        soma_nodes.push(child_id);
        curr_id = child_id;
    }

    // now we can start building up the sections that we will eventually return
    sections.push(Section {
        id: 0,
        parent_id: None,
        children_ids: vec![],
        length: soma_length,
        mean_diam: soma_sum_diam / soma_nodes.len() as f64,
        swc_nodes: soma_nodes,
    });

    // ----------------------------------------------------------------
    // Phase 2: Create sections from the Dendrites and Axons
    // ----------------------------------------------------------------
    let mut starter_nodes: Vec<u64> = Vec::new();

    for node in sorted_nodes {
        if is_soma_type(node) {
            continue; // soma nodes already handled in Phase 1
        }

        let parent_node = node_map[&node.parent_id];
        let parent_is_soma = is_soma_type(parent_node);
        let parent_non_soma_children = filtered_children(parent_child_map, node.parent_id)
            .into_iter()
            .filter(|&cid| !is_soma_type(node_map[&cid]))
            .count();

        if parent_is_soma || parent_non_soma_children > 1 {
            starter_nodes.push(node.node_id);
        }
    }

    for (i, &starter_id) in starter_nodes.iter().enumerate() {
        let sec_id = (i + 1) as u64; // section 0 is always soma
        let mut curr_id = starter_id;
        let mut swc_nodes = vec![curr_id];
        // Length begins with the edge from parent to this starter node.
        let parent_id = node_map[&curr_id].parent_id;
        let mut length = compute_length(node_map[&curr_id], node_map[&parent_id]);
        let mut sum_diam = node_map[&curr_id].radius * 2.0;

        loop {
            node_to_section.insert(curr_id, sec_id);
            let children = filtered_children(parent_child_map, curr_id);

            if children.is_empty() || children.len() > 1 {
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
            mean_diam: sum_diam / swc_nodes.len() as f64,
            swc_nodes,
        });
    }

    // ----------------------------------------------------------------
    // Phase 3: Wire topology.
    // Fill in the parent IDs (forwards) and then the children IDs (backwards)
    // ----------------------------------------------------------------
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
