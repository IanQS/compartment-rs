use itertools::Itertools;
use log::{info, warn};
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader};

/// We use the CNIC spec, as per: http://www.neuronland.org/NLMorphologyConverter/MorphologyFormats/SWC/Spec.html
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
enum StructureIdentifier {
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
struct Node {
    node_id: u64,
    structured_identifier: StructureIdentifier,
    x_pos: f64,
    y_pos: f64,
    z_pos: f64,
    radius: u64,
    parent_id: u64,
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
) -> Result<Vec<Node>, String> {
    let f = File::open(read_path).unwrap();

    let lines: Vec<String> = BufReader::new(f)
        .lines()
        .filter_map(|line| line.ok())
        .filter(|line| !line.starts_with('#'))
        .collect();

    let nodes_vec: Vec<Node> = lines
        .iter()
        .map(|line| {
            let mut v = line.split_whitespace();
            let node_id = v.next().unwrap().parse::<u64>().unwrap() + 1;
            let structured_identifier: StructureIdentifier =
                v.next().unwrap().parse::<u8>().unwrap().into();
            let node = Node {
                node_id,
                structured_identifier,
                x_pos: v.next().unwrap().parse::<f64>().unwrap(),
                y_pos: v.next().unwrap().parse::<f64>().unwrap(),
                z_pos: v.next().unwrap().parse::<f64>().unwrap(),
                radius: v.next().unwrap().parse::<u64>().unwrap(),
                parent_id: (v.next().unwrap().parse::<i64>().unwrap() + 1) as u64,
            };
            if node.radius == 0 && emit_warnings.unwrap_or(true) {
                warn!(
                    "Zero-radius for section ID: {} of type: {:?}",
                    node_id, structured_identifier
                );
                if structured_identifier != StructureIdentifier::EndPoint && strict.unwrap_or(false)
                {
                    return Err("Zero-radius for non-endpoint");
                } else {
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
    let root = nodes_vec.iter().find(|n| n.parent_id == 0)
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

    // Create old_id -> new_id mapping (sequential starting at 1)
    let mut old_to_new_id: HashMap<u64, u64> = HashMap::new();
    for (new_id, old_id) in sorted_node_ids.iter().enumerate() {
        old_to_new_id.insert(*old_id, (new_id + 1) as u64);
    }

    // Track statistics
    let mut zero_radius_count: HashMap<String, usize> = HashMap::new();
    let mut label_breakdown: HashMap<String, usize> = HashMap::new();

    // Remap nodes with new sequential IDs and fix radii
    let mut remapped_nodes: Vec<Node> = sorted_node_ids
        .iter()
        .map(|old_id| {
            let mut node = nodes_by_id[old_id];
            let new_id = old_to_new_id[old_id];

            // Remap parent ID
            node.parent_id = if node.parent_id == 0 {
                0
            } else {
                *old_to_new_id.get(&node.parent_id).unwrap_or(&0)
            };

            node.node_id = new_id;

            // Fix radius if needed
            let type_str = format!("{:?}", node.structured_identifier);
            if node.radius == 0 {
                *zero_radius_count.entry(type_str.clone()).or_insert(0) += 1;
                node.radius = 1;
            }

            // Track label statistics
            *label_breakdown.entry(type_str).or_insert(0) += 1;

            node
        })
        .collect();

    // Write to file if requested
    if let Some(output_path) = write_path {
        let mut output = String::new();
        output.push_str("# Processed SWC file\n");

        for node in &remapped_nodes {
            let parent_id = if node.parent_id == 0 {
                -1i64
            } else {
                (node.parent_id - 1) as i64
            };

            output.push_str(&format!(
                "{} {} {:.2} {:.2} {:.2} {} {}\n",
                node.node_id - 1,
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
        info!("SWC Label Convention: 0=undefined, 1=soma, 2=axon, 3=basal dendrite, 4=apical dendrite, 5=fork, 6=end");
        info!("Fixed zero-radius points by type: {:?}", zero_radius_count);
    }

    info!("Node type breakdown: {:?}", label_breakdown);

    Ok(remapped_nodes)
}
