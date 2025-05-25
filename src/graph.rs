use std::{collections::HashMap, fmt::Display, ops::Deref, rc::Rc};

use anyhow::{Result, anyhow};
use petgraph::{
    Graph,
    graph::NodeIndex,
    visit::{Dfs, Visitable},
};

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Target {
    name: String,
    flavor: String,
}

impl Target {
    pub fn new(name: String, flavor: String) -> Self {
        Target { name, flavor }
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.flavor)
    }
}

// petgraph has a whole notion of only using copy-able indices for their graph
// everything happens in the form of `NodeIndex`, its hard to get what "index" some node is natively from petgraph
// as such, we manage indices ourselves in our fat structure
// for nodes themselves, i dont care what data goes into them (it could simply be ()) while we keep track of true nodes
// nodes in our hashmap
// I would love to use GraphMap directly, but i cant implement `Copy` on my package
// I could keep track of some random unique integer for that, but thats the same work as simply storing indices
// we make a ridiculous number of clone calls for `NodeIndex`, this is fine as its just a u32
pub struct TargetGraph {
    // the main inner graph
    // make this acyclic
    inner: Graph<(), ()>,

    // used for lookup
    // for now im keeping copies here, its hard to wrap
    // my head around what would work
    target_by_index: HashMap<Rc<Target>, NodeIndex>,

    // honestly at this point, i dont see a point of using this stupid lib
    // ive to keep both mappings to make it any reasonably fast
    index_by_target: HashMap<NodeIndex, Rc<Target>>,
}

impl TargetGraph {
    pub fn new() -> Self {
        TargetGraph {
            inner: Graph::new(),
            target_by_index: HashMap::new(),
            index_by_target: HashMap::new(),
        }
    }

    pub fn contains_node(&self, node: &Target) -> bool {
        let index = self.target_by_index.get(node);
        match index {
            None => false,
            Some(v) => match self.inner.node_weight(v.clone()) {
                None => false,
                Some(_) => true,
            },
        }
    }

    pub fn add_node(&mut self, node: Target) {
        if self.target_by_index.contains_key(&node) {
            return;
        }
        let index = self.inner.add_node(());
        let node = Rc::new(node);
        self.index_by_target.insert(index, Rc::clone(&node));
        self.target_by_index.insert(node, index);
    }

    pub fn add_edge(&mut self, src: &Target, dest: &Target) -> Result<()> {
        let s = self.get_cloned_node_index(src)?;
        let d = self.get_cloned_node_index(dest)?;
        if self.inner.contains_edge(s.clone(), d.clone()) {
            return Ok(());
        }
        self.inner.add_edge(s, d, ());
        Ok(())
    }

    fn get_cloned_node_index(&self, target: &Target) -> Result<NodeIndex> {
        let d = self
            .target_by_index
            .get(target)
            .ok_or(anyhow!(
                "error while adding edge: dest index: {:?} not found",
                target
            ))?
            .clone();
        Ok(d)
    }

    pub fn rdeps(&self, targets: Vec<&Target>) -> Result<Vec<Target>> {
        let mut indices = Vec::new();
        for t in targets {
            indices.push(self.get_cloned_node_index(t)?);
        }
        let mut dfs = Dfs::from_parts(indices, self.inner.visit_map());
        let mut res = Vec::new();
        while let Some(next_index) = dfs.next(&self.inner) {
            let node =self.index_by_target.get(&next_index).expect(
                &format!("corrupted graph state, `index_by_target` did not contain an index we got from dfs in graph, index={:?}", next_index)
            );

            res.push(node.deref().clone());
        }

        Ok(res)
    }
}

impl Display for TargetGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node_idx in self.inner.node_indices() {
            let node = self.index_by_target.get(&node_idx).expect("corrupted graph state");
            write!(f, "{} -> ", node)?;
            
            let mut neighbors = Vec::new();
            for neighbor_idx in self.inner.neighbors(node_idx) {
                let neighbor = self.index_by_target.get(&neighbor_idx).expect("corrupted graph state");
                neighbors.push(format!("{}", neighbor));
            }
            
            writeln!(f, "[{}]", neighbors.join(", "))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{Target, TargetGraph};

    #[test]
    fn test_rdeps() {
        let mut g = TargetGraph::new();

        // atleast create a reasonably big transitive graph

        let qsync_stream = Target::new(String::from("qsync_stream"), String::from("cargo"));
        let image_manager = Target::new(String::from("image_manager"), String::from("cargo"));
        let qure_dicom_utils = Target::new(String::from("qure_dicom_utils"), String::from("cargo"));
        let qxr = Target::new(String::from("qxr"), String::from("cargo"));
        let qxr_reports = Target::new(String::from("qxr_reports"), String::from("cargo"));
        let qer = Target::new(String::from("qer"), String::from("cargo"));
        let qer_reports = Target::new(String::from("qer_reports"), String::from("cargo"));
        let qureapi = Target::new(String::from("qureapi"), String::from("cargo"));
        let cathode = Target::new(String::from("cathode"), String::from("cargo"));

        g.add_node(image_manager.clone());
        g.add_node(qsync_stream.clone());
        g.add_node(qure_dicom_utils.clone());
        g.add_node(qxr.clone());
        g.add_node(qxr_reports.clone());
        g.add_node(qer.clone());
        g.add_node(qer_reports.clone());
        g.add_node(qureapi.clone());
        g.add_node(cathode.clone());

        // qsync_stream -> image_manager
        g.add_edge(&qsync_stream, &image_manager).unwrap();

        // qxr -> qxr_reports -> cathode
        // qxr -> cathode
        // qxr -> qureapi
        g.add_edge(&qxr, &qxr_reports).unwrap();
        g.add_edge(&qxr_reports, &cathode).unwrap();
        g.add_edge(&qxr, &cathode).unwrap();
        g.add_edge(&qxr, &qureapi).unwrap();

        // qer -> qer_reports -> qureapi
        // qer -> qureapi
        g.add_edge(&qer, &qer_reports).unwrap();
        g.add_edge(&qer_reports, &qureapi).unwrap();
        g.add_edge(&qer, &qureapi).unwrap();

        // qure_dicom_utils -> qxr
        // qure_dicom_utils -> qer
        g.add_edge(&qure_dicom_utils, &qxr).unwrap();
        g.add_edge(&qure_dicom_utils, &qer).unwrap();

        let res = g.rdeps(vec![&qxr]).unwrap();
        assert!(res.contains(&qureapi));
        assert!(res.contains(&qxr));
        assert!(res.contains(&qxr_reports));
        assert!(res.contains(&cathode));
        assert_eq!(res.len(), 4);

        // this graph does not contain any qxr specific stuff
        let res = g.rdeps(vec![&qer]).unwrap();
        assert!(res.contains(&qureapi));
        assert!(res.contains(&qer));
        assert!(res.contains(&qer_reports));
        assert_eq!(res.len(), 3);

        // totally different graph
        let res = g.rdeps(vec![&qsync_stream]).unwrap();
        assert!(res.contains(&qsync_stream));
        assert!(res.contains(&image_manager));
        assert_eq!(res.len(), 2);
    }
}
