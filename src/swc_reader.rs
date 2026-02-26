use itertools::Itertools;
use log::{info, warn};
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};

/// We use the CNIC spec, as per: http://www.neuronland.org/NLMorphologyConverter/MorphologyFormats/SWC/Spec.html
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub(crate) enum StructureIdentifier {
    Undefined,
    Soma,
    Axon,
    BasalDendrite,
    ApicalDendrite,
    ForkPoint,
    EndPoint,
    Custom,
}

impl From<u8> for StructureIdentifier {
    fn from(v: u8) -> Self {
        match v {
            0 => StructureIdentifier::Undefined,
            1 => StructureIdentifier::Soma,
            2 => StructureIdentifier::Axon,
            3 => StructureIdentifier::BasalDendrite,
            4 => StructureIdentifier::ApicalDendrite,
            5 => StructureIdentifier::ForkPoint,
            6 => StructureIdentifier::EndPoint,
            _ => StructureIdentifier::Custom,
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct Node {
    pub node_id: u64,
    pub structured_identifier: StructureIdentifier,
    pub x_pos: f64,
    pub y_pos: f64,
    pub z_pos: f64,
    pub radius: f64,
    pub parent_id: u64,
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
    }
}
impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_id.hash(state);
    }
}

/// Reads in swc from `read_path` and returns the generated compartment skeleton
///   If a `write_path` is given, we spit out the processed, sorted, file there,
///   with the comments at the start stripped out
/// Optionally emits warnings for:
///   - zero-radius points
/// Strict mode:
///   - if any of the above warnings are hit, we terminate immediately
///
/// Based on https://en.wikipedia.org/wiki/Topological_sorting#Depth-first_search
/// For Flywire.ai skeletons, seems they only mark out:
/// # 0 = undefined, 1 = soma, 5 = fork point, 6 = end point
pub fn swc_reader(
    read_path: String,
    emit_warnings: Option<bool>,
    strict: Option<bool>,
    write_path: Option<String>,
) -> Result<(Vec<Node>, HashMap<u64, Vec<u64>>, HashMap<u64, Vec<u64>>), String> {
    let f = File::open(read_path).unwrap(); //.map_err(|x| format!("No such read path"));

    let lines: Vec<String> = BufReader::new(f)
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| !line.starts_with('#'))
        .collect();

    let nodes_vec: Vec<Node> = lines
        .iter()
        .map(|line| {
            let mut v = line.split_whitespace();
            let node_id = v.next().unwrap().parse::<u64>().unwrap();
            let structured_identifier: StructureIdentifier =
                v.next().unwrap().parse::<u8>().unwrap().into();

            let x_pos = v.next().unwrap().parse::<f64>().unwrap();
            let y_pos = v.next().unwrap().parse::<f64>().unwrap();
            let z_pos = v.next().unwrap().parse::<f64>().unwrap();
            let radius = v.next().unwrap().parse::<f64>().unwrap();

            // Parse parent_id: -1 in file becomes 0 (temporary, will be self-referencing for root)
            let parent_id_raw = v.next().unwrap().parse::<i64>().unwrap();
            let parent_id = if parent_id_raw == -1 { 0 } else { parent_id_raw as u64 };
            let node = Node {
                node_id,
                structured_identifier,
                x_pos,
                y_pos,
                z_pos,
                radius,
                parent_id,
            };

            if node.radius == 0.0 && emit_warnings.unwrap_or(true) {
                warn!(
                    "Zero-radius for section ID: {} of type: {:?}",
                    node_id, structured_identifier
                );
                if structured_identifier != StructureIdentifier::EndPoint && strict.unwrap_or(false)
                {
                    return Err("Zero-radius for non-endpoint");
                }
            }
            Ok(node)
        })
        .collect::<Result<Vec<Node>, _>>()?;

    // Quick debug logs for the count of the types
    let accum_types: HashMap<StructureIdentifier, usize> = nodes_vec
        .iter()
        .map(|&node| node.structured_identifier)
        .counts();
    for el in accum_types {
        info!("{:?} - #{:?}", el.0, el.1);
    }

    // Create lookup map: node_id -> Node
    let nodes_by_id: HashMap<u64, Node> = nodes_vec.iter().map(|n| (n.node_id, *n)).collect();

    ////////////////////////
    // BFS traversal for topological order
    ////////////////////////
    // Construct mapping from parent to children for the BFS
    let mut children: HashMap<u64, Vec<u64>> = HashMap::new();
    for n in &nodes_vec {
        children.entry(n.parent_id).or_default().push(n.node_id);
    }

    // Find root node (parent_id == 0)
    let root = nodes_vec
        .iter()
        .find(|n| n.parent_id == 0)
        .ok_or("No root node found (parent_id == 0)")?;

    let mut sorted_node_ids: Vec<u64> = Vec::new();
    let mut queue: VecDeque<u64> = VecDeque::new();
    queue.push_back(root.node_id);
    let mut visited: HashSet<u64> = HashSet::new();

    while let Some(node_id) = queue.pop_front() {
        if visited.contains(&node_id) {
            warn!("Cycle detected at {}", node_id);
            continue;
        }
        visited.insert(node_id);
        sorted_node_ids.push(node_id);

        // Add children to queue
        if let Some(child_ids) = children.get(&node_id) {
            for &child_id in child_ids {
                if !visited.contains(&child_id) {
                    queue.push_back(child_id);
                }
            }
        }
    }

    // Create old_id -> new_id mapping (sequential starting at 0)
    let mut old_to_new_id: HashMap<u64, u64> = HashMap::new();
    for (new_id, old_id) in sorted_node_ids.iter().enumerate() {
        old_to_new_id.insert(*old_id, new_id as u64);
    }

    // Track statistics
    let mut zero_radius_count: HashMap<String, usize> = HashMap::new();
    let mut label_breakdown: HashMap<String, usize> = HashMap::new();

    // Map forward from the soma -> dendrites
    let mut parent_child_map: HashMap<u64, Vec<u64>> = HashMap::new();
    // Map backward from dendrites -> Soma
    let mut child_parent_map: HashMap<u64, Vec<u64>> = HashMap::new();
    // Remap nodes with new sequential IDs and fix radii
    let remapped_nodes: Vec<Node> = sorted_node_ids
        .iter()
        .map(|old_id| {
            let mut node = nodes_by_id[old_id];
            let new_id = old_to_new_id[old_id];

            node.node_id = new_id;

            // Remap parent ID: root node becomes self-referencing
            node.parent_id = if node.parent_id == 0 {
                new_id // Root points to itself
            } else {
                *old_to_new_id.get(&node.parent_id).unwrap_or(&0)
            };

            // Fix radius if needed
            let type_str = format!("{:?}", node.structured_identifier);
            if node.radius == 0.0 {
                *zero_radius_count.entry(type_str.clone()).or_insert(0) += 1;
                node.radius = 1.0;
            }

            // Track label statistics
            *label_breakdown.entry(type_str).or_insert(0) += 1;

            // parent_child_map.insert(node.parent_id, node.node_id);
            parent_child_map
                .entry(node.parent_id)
                .or_insert_with(Vec::new)
                .push(node.node_id);
            child_parent_map
                .entry(node.node_id)
                .or_insert_with(Vec::new)
                .push(node.parent_id);

            node
        })
        .collect();

    // Write to file if requested
    if let Some(output_path) = write_path {
        let mut output = String::new();
        output.push_str("# Processed SWC file\n");

        for node in &remapped_nodes {
            // Root node (self-referencing) should be written as -1
            let parent_id = if node.parent_id == node.node_id {
                -1i64
            } else {
                node.parent_id as i64
            };

            output.push_str(&format!(
                "{} {} {:.2} {:.2} {:.2} {} {}\n",
                node.node_id,
                match node.structured_identifier {
                    StructureIdentifier::Undefined => 0,
                    StructureIdentifier::Soma => 1,
                    StructureIdentifier::Axon => 2,
                    StructureIdentifier::BasalDendrite => 3,
                    StructureIdentifier::ApicalDendrite => 4,
                    StructureIdentifier::ForkPoint => 5,
                    StructureIdentifier::EndPoint => 6,
                    StructureIdentifier::Custom => 7,
                },
                node.x_pos,
                node.y_pos,
                node.z_pos,
                node.radius,
                parent_id
            ));
        }

        fs::write(output_path, output).unwrap();
    }

    // Log summary
    info!("Processed {} nodes", remapped_nodes.len());

    if !zero_radius_count.is_empty() {
        info!(
            "SWC Label Convention: 0=undefined, 1=soma, 2=axon, 3=basal dendrite, 4=apical dendrite, 5=fork, 6=end"
        );
        info!("Fixed zero-radius points by type: {:?}", zero_radius_count);
    }

    info!("Node type breakdown: {:?}", label_breakdown);

    Ok((remapped_nodes, parent_child_map, child_parent_map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swc_reader_basic() {
        let read_path = "data/basic.swc".to_string();
        let emit_warnings = Some(true);
        let strict = Some(true);
        let write_path = None;

        let result = swc_reader(read_path, emit_warnings, strict, write_path);
        assert!(result.is_ok());

        let (nodes, parent_child_map, child_parent_map) = result.unwrap();
        println!("Nodes length: {}", nodes.len());
        println!("Nodes: {:?}", nodes.iter().map(|n| n.node_id).collect::<Vec<_>>());
        
        // basic.swc has 5 nodes
        assert_eq!(nodes.len(), 5);

        // Check assigned IDs
        for i in 0..5 {
            assert_eq!(nodes[i].node_id, i as u64);
        }

        // Root node self-referencing check
        let root_node = &nodes[0];
        assert_eq!(root_node.parent_id, root_node.node_id);

        // Check tree structure relationships based on old IDs mapped to new IDs
        // old id 1 (new 0) is root
        // old id 2 (new 1) -> old id 1 (new 0)
        // old id 3 (new 2) -> old id 1 (new 0)
        // old id 4 (new 3) -> old id 1 (new 0)
        // old id 5 (new 4) -> old id 4 (new 3)
        assert_eq!(nodes[1].parent_id, 0); // node 2 parent is 1
        assert_eq!(nodes[2].parent_id, 0); // node 3 parent is 1
        assert_eq!(nodes[3].parent_id, 0); // node 4 parent is 1
        assert_eq!(nodes[4].parent_id, 3); // node 5 parent is 4

        // Check map sizes
        // parent 0 -> children 0 (self), 1, 2, 3
        // parent 3 -> child 4
        assert_eq!(parent_child_map.get(&0).unwrap().len(), 4);
        assert!(parent_child_map.get(&0).unwrap().contains(&0));
        assert!(parent_child_map.get(&0).unwrap().contains(&1));
        assert!(parent_child_map.get(&0).unwrap().contains(&2));
        assert!(parent_child_map.get(&0).unwrap().contains(&3));
        
        assert_eq!(parent_child_map.get(&3).unwrap().len(), 1);
        assert!(parent_child_map.get(&3).unwrap().contains(&4));

        assert_eq!(child_parent_map.get(&1).unwrap()[0], 0);
        assert_eq!(child_parent_map.get(&2).unwrap()[0], 0);
        assert_eq!(child_parent_map.get(&3).unwrap()[0], 0);
        assert_eq!(child_parent_map.get(&4).unwrap()[0], 3);
    }
}

