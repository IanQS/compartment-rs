use std::collections::HashMap;
use std::f64::consts::PI;

use crate::channels::Channel;

use crate::sections::coalesce_into_sections;
use crate::swc_reader::Node;

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
    fn d_lambda_rule(self, frequency: f64, d_lambda: f64) -> Compartments {
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

    fn attach_stimuli(self, _stimulus: Vec<f64>) -> () {
        todo!(
            "Attach a stimuli pattern to a specific compartment. HAS to be of equal length to T/dt"
        )
    }

    fn simulate(_dt: f64, _t: f64) -> () {
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

        assert_eq!(compartments.components.len(), 4);
        assert_eq!(compartments.components[0].name, "Soma");
        // Verify some properties are defaults
        assert_eq!(compartments.components[0].axial_resistivity, 100.0);
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
        let original_count = compartments_1.components.len();
        let refined_compartments_1 = compartments_1.d_lambda_rule(100.0, 0.1);
        assert!(refined_compartments_1.components.len() >= original_count);

        let compartments_2 =
            Compartments::from_sorted_nodes(nodes, parent_child_map, child_parent_map);
        let refined_compartments_2 = compartments_2.d_lambda_rule(100.0, 0.01);
        assert!(refined_compartments_2.components.len() >= refined_compartments_1.components.len());
    }
}
