use std::collections::HashMap;

use crate::channels::Channel;

use crate::swc_reader::Node;

#[derive(Default)]
pub struct Compartment {
    pub(crate) name: String,             // Name string for easier identification
    idx: u64,                 // Index into our compartments list
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
fn compute_length(curr: &Node, other: &Node) -> f64{
    let x_diff = square(curr.x_pos - other.x_pos);
    let y_diff = square(curr.y_pos - other.y_pos);
    let z_diff = square(curr.z_pos - other.z_pos);
    return (x_diff + y_diff + z_diff).sqrt()
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
        // Add a dummy root to make it so that the soma (element 1) maps correctly
        // and has the parent being the dummy
        let dummy_root = Compartment{
            name: "Dummy Root".to_owned(),
            idx: 0,
            parent_idxs: Vec::new(),
            children_idxs: Vec::new(),
            length: 0.0,
            diam: 0.0,
            channel: Channel::default()
        };

        // First pass - we populate the network "going forward" to fill up the parents
        components.push(dummy_root);
        for (i, node) in sorted_nodes.iter().enumerate(){
            let name;
            if i == 0{
                name = "Compartment: 1 (Soma)".to_owned();
            } else {
                name = format!("Compartment: {}", (i+1).to_string());
            }

            // Compute length from parent
            let length = if node.parent_id == 0 {
                // Soma: parent is dummy root, no meaningful length between them
                0.0
            } else {
                // Look up parent by its node_id, not by direct indexing
                let parent_idx = node_id_to_idx[&node.parent_id];
                let parent_node = &sorted_nodes[parent_idx];
                compute_length(node, parent_node)
            };

            let parents = child_parent_map
                .get(&node.node_id)
                .cloned()
                .unwrap_or_default();
            let children= parent_child_map
                .get(&node.node_id)
                .cloned()
                .unwrap_or_default();

            let compartment = Compartment{
                name,
                idx: components.len() as u64,
                parent_idxs: parents,
                children_idxs: children,
                length,
                diam: node.radius * 2.0,
                channel: Channel::default()
            };

            components.push(compartment);
        }

        return Compartments {components}
    }

    ///# Reasonable default values for most models.
    /// Taken from https://jaxley.readthedocs.io/en/stable/how_to_guide/set_ncomp.html
    // frequency = 100.0
    // d_lambda = 0.1  # Larger -> more coarse-grained.

    // for branch in cell.branches:
    //     diameter = 2 * branch.nodes["radius"].to_numpy()[0]
    //     c_m = branch.nodes["capacitance"].to_numpy()[0]
    //     r_a = branch.nodes["axial_resistivity"].to_numpy()[0]
    //     l = branch.nodes["length"].to_numpy()[0]

    //     lambda_f = 1e5 * np.sqrt(diameter / (4 * np.pi * frequency * c_m * r_a))
    //     ncomp = int((l / (d_lambda * lambda_f) + 0.9) / 2) * 2 + 1
    //     branch.set_ncomp(ncomp, initialize=False)

    fn d_lambda_rule(mut self, frequency: f64, d_lambda: f64) -> Compartments {
        let new_compartments: Vec<Compartment>= Vec::new();
        for compartment in self.components{
            
        }

        return Compartments {components: new_compartments}

    }

    fn attach_stimuli(mut compartments, stimulus: Vec<f64>) -> {
        todo!("Attach a stimuli pattern to a specific compartment. HAS to be of equal length to T/dt")
    }

    fn simulate(dt: f64, T:f64) -> (){
        todo!("")
    }
}

