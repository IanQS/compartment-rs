use std::collections::HashMap;
use std::f64::consts::PI;

use crate::channels::Channel;

use crate::swc_reader::Node;

#[derive(Default)]
pub struct Compartment {
    pub(crate) name: String, // Name string for easier identification
    idx: u64,                // Index into our compartments list
    parent_idxs: Vec<u64>,   // Index into our compartments lists
    children_idxs: Vec<u64>, // Index into our compartments lists

    length: f64,
    diam: f64,

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
        child_parent_map: HashMap<u64, Vec<u64>>,
    ) -> Compartments {
        // Create lookup: node_id -> index in sorted_nodes
        let node_id_to_idx: HashMap<u64, usize> = sorted_nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| (node.node_id, idx))
            .collect();

        let mut components = Vec::new();

        // Build compartments from nodes (Node IDs are 0-based)
        // Root node (soma) is self-referencing: parent_id == node_id
        for (i, node) in sorted_nodes.iter().enumerate() {
            let name = if i == 0 {
                "Soma".to_owned()
            } else {
                format!("Compartment {}", i + 1)
            };

            // Compute length from parent
            let length = if node.parent_id == node.node_id {
                // Root node (soma): it's its own parent, no length
                0.0
            } else {
                // Look up parent by its node_id, not by direct indexing
                let parent_idx = node_id_to_idx[&node.parent_id];
                let parent_node = &sorted_nodes[parent_idx];
                compute_length(node, parent_node)
            };

            let parents: Vec<u64> = child_parent_map
                .get(&node.node_id)
                .cloned()
                .unwrap_or_default();

            let children: Vec<u64> = parent_child_map
                .get(&node.node_id)
                .cloned()
                .unwrap_or_default();

            let compartment = Compartment {
                name,
                idx: i as u64, // Component index (0-based)
                parent_idxs: parents,
                children_idxs: children,
                length,
                diam: node.radius * 2.0,
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
            let r_a = compartment.channel.axial_resistivity;
            let c_m = compartment.channel.capacitance;
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
        assert_eq!(compartments.components[1].name, "Compartment 2");

        // Soma length is 0.0
        assert_eq!(compartments.components[0].length, 0.0);

        // Node 1 (old 2): xyz = 3 4 5, Node 0 (old 1): xyz = 0 0 0
        // length = sqrt(3^2 + 4^2 + 5^2) = sqrt(50) = 7.071...
        assert!((compartments.components[1].length - 50.0_f64.sqrt()).abs() < 1e-6);

        // Verify some channel properties are defaults
        assert_eq!(
            compartments.components[0].channel.axial_resistivity,
            Channel::default().axial_resistivity
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
