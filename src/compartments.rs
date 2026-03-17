use std::collections::HashMap;
use std::f64::consts::PI;

use crate::channels::Channel;

use crate::swc_reader::Node;

#[derive(Debug, Clone)]
pub struct Section {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub children_ids: Vec<u64>,
    pub length: f64,
    pub mean_diam: f64,
    pub swc_nodes: Vec<u64>,
}

pub fn coalesce_into_sections(
    sorted_nodes: &[Node],
    parent_child_map: &HashMap<u64, Vec<u64>>,
) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut starter_nodes = Vec::new();

    // 1. Identify starter nodes
    for node in sorted_nodes {
        let is_soma = node.parent_id == node.node_id;
        if is_soma {
            starter_nodes.push(node.node_id);
            continue;
        }

        let parent_id = node.parent_id;
        let parent_node = sorted_nodes.iter().find(|n| n.node_id == parent_id).unwrap();
        let is_parent_soma = parent_node.parent_id == parent_node.node_id;
        let parent_children = parent_child_map.get(&parent_id).map(|c| c.len()).unwrap_or(0);

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

        let is_soma = node_map[&starter_id].parent_id == starter_id;
        if !is_soma {
            let parent_id = node_map[&curr_id].parent_id;
            length += compute_length(node_map[&curr_id], node_map[&parent_id]);
        }

        loop {
            node_to_section.insert(curr_id, sec_id);
            let children = parent_child_map.get(&curr_id).cloned().unwrap_or_default();

            if children.is_empty() || children.len() > 1 || (curr_id == starter_id && is_soma) {
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

    // Clone to avoid borrow checker issues when mutating children
    let sections_clone = sections.clone();
    for section in &mut sections {
        if let Some(parent_id) = section.parent_id {
            // Find the parent section in the original vec and push child
            let parent_idx = sections_clone.iter().position(|s| s.id == parent_id).unwrap();
            // Since we can't easily mutate another element while iterating, 
            // we'll do this in a separate pass.
        }
    }
    
    // Fix wiring step
    for child_idx in 0..sections.len() {
        if let Some(parent_idx) = sections[child_idx].parent_id {
            // parent_idx is the section id, which matches its index in the vector
            sections[parent_idx as usize].children_ids.push(sections[child_idx].id);
        }
    }

    sections
}

#[derive(Default)]
pub struct Compartment {
    pub(crate) name: String, // Name string for easier identification
    idx: u64,                // Index into our compartments list
    parent_idxs: Vec<u64>,   // Index into our compartments lists
    children_idxs: Vec<u64>, // Index into our compartments lists

    length: f64,
    diam: f64,
    pub capacitance: f64,
    pub axial_resistivity: f64,

    channel: Channel,
}

impl Compartment {
    fn set_channel() -> () {}
}

pub struct Compartments {
    pub components: Vec<Compartment>,
}

fn square(x: f64) -> f64 {
    x * x
}

/// Assumes simple direct path between the nodes
fn compute_length(curr: &Node, other: &Node) -> f64 {
    let x_diff = square(curr.x_pos - other.x_pos);
    let y_diff = square(curr.y_pos - other.y_pos);
    let z_diff = square(curr.z_pos - other.z_pos);
    (x_diff + y_diff + z_diff).sqrt()
}

impl Compartments {
    fn from_sorted_nodes(
        sorted_nodes: Vec<Node>,
        parent_child_map: HashMap<u64, Vec<u64>>,
        _child_parent_map: HashMap<u64, Vec<u64>>,
    ) -> Compartments {
        let sections = coalesce_into_sections(&sorted_nodes, &parent_child_map);
        let mut components = Vec::with_capacity(sections.len());

        for section in sections {
            let name = if section.parent_id.is_none() {
                "Soma".to_owned()
            } else {
                format!("Compartment {}", section.id)
            };

            let parents = match section.parent_id {
                Some(pid) => vec![pid],
                None => vec![],
            };

            let compartment = Compartment {
                name,
                idx: section.id,
                parent_idxs: parents,
                children_idxs: section.children_ids,
                length: section.length,
                diam: section.mean_diam,
                capacitance: 1.0,         // typical default 1.0 uF/cm^2
                axial_resistivity: 100.0, // typical default 100.0 ohm*cm
                channel: Channel::default(),
            };

            components.push(compartment);
        }

        Compartments { components }
    }

    ///# Reasonable default values for most models.
    /// Taken from https://jaxley.readthedocs.io/en/stable/how_to_guide/set_ncomp.html
    /// A-> [B] -> C
    /// becomes:
    /// A -> [B_1 -> B_2, ... B_N] -> C
    /// The authors there used frequency=100 and d_lambda=0.1
    fn d_lambda_rule(mut self, frequency: f64, d_lambda: f64) -> Compartments {
        let mut new_compartments: Vec<Compartment> = Vec::new();
        let mut current_idx: u64 = 0;

        for compartment in self.components {
            let r_a = compartment.axial_resistivity;
            let c_m = compartment.capacitance;
            let lambda_f = 1e5 * (compartment.diam / (4.0 * PI * frequency * c_m * r_a)).sqrt();
            let n_comp = (((compartment.length / (d_lambda * lambda_f) + 0.9) / 2.0).floor() * 2.0
                + 1.0) as u64;

            let new_length = compartment.length / n_comp as f64;

            // Store indices of the new sub-compartments for this segment
            if n_comp == 1 {
                let new_comp = Compartment {
                    idx: current_idx,
                    ..compartment
                };
                new_compartments.push(new_comp);
                current_idx += 1;
            } else {
                for i in 0..n_comp {
                    let name = format!("{}_{}", compartment.name, i);
                    let idx = current_idx;
                    current_idx += 1;

                    let (parent_idxs, children_idxs) = match i {
                        0 => {
                            // First sub-compartment: keeps original parents
                            (compartment.parent_idxs.clone(), vec![idx + 1])
                        }
                        j if j == n_comp - 1 => {
                            // Last sub-compartment: keeps original children
                            (vec![idx - 1], compartment.children_idxs.clone())
                        }
                        _ => {
                            // Middle sub-compartments: chain together
                            (vec![idx - 1], vec![idx + 1])
                        }
                    };

                    let new_comp = Compartment {
                        name,
                        idx,
                        parent_idxs,
                        children_idxs,
                        length: new_length,
                        diam: compartment.diam,
                        capacitance: compartment.capacitance,
                        axial_resistivity: compartment.axial_resistivity,
                        channel: compartment.channel,
                    };

                    new_compartments.push(new_comp);
                }
            }
        }

        Compartments {
            components: new_compartments,
        }
    }

    fn attach_stimuli(mut self, stimulus: Vec<f64>) -> () {
        todo!(
            "Attach a stimuli pattern to a specific compartment. HAS to be of equal length to T/dt"
        )
    }

    fn simulate(dt: f64, T: f64) -> () {
        todo!("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swc_reader::swc_reader;

    #[test]
    fn test_compartments_from_sorted_nodes() {
        let (nodes, parent_child_map, child_parent_map) =
            swc_reader("data/basic.swc".to_string(), Some(true), Some(false), None).unwrap();

        let compartments =
            Compartments::from_sorted_nodes(nodes, parent_child_map, child_parent_map);

        assert_eq!(compartments.components.len(), 5);
        assert_eq!(compartments.components[0].name, "Soma");
        // Component 1 should correspond to old nodes 2,3,4 coalesced
        let node1 = nodes[1];
        let node2 = nodes[2];
        let node3 = nodes[3];
        let node4 = nodes[4];

        // length = dist(1->2) + dist(2->3) + dist(3->4) where old id 1 is soma
        // basic.swc:
        // 1 1 0 0 0 1 -1 (Soma)
        // 2 3 3 4 5 1 1 (Length ~ 7.071 - wait, we just add lengths)

        // Verify some properties are defaults
        assert_eq!(
            compartments.components[0].axial_resistivity,
            100.0
        );
    }

    #[test]
    fn test_compartments_d_lambda_rule() {
        let (nodes, parent_child_map, child_parent_map) = swc_reader(
            "data/morph_minimal.swc".to_string(),
            Some(true),
            Some(false),
            None,
        )
        .unwrap();

        let compartments_1 = Compartments::from_sorted_nodes(
            nodes.clone(),
            parent_child_map.clone(),
            child_parent_map.clone(),
        );
        let refined_compartments_1 = compartments_1.d_lambda_rule(100.0, 0.1);
        assert_eq!(refined_compartments_1.components.len(), 20);

        let compartments_2 =
            Compartments::from_sorted_nodes(nodes, parent_child_map, child_parent_map);
        let refined_compartments_2 = compartments_2.d_lambda_rule(100.0, 0.01);
        assert_eq!(refined_compartments_2.components.len(), 20);
    }
}
