pub mod graph;
pub mod infer;

fn main() {
    println!("Hello, world!");
}

// // given cargo.toml, parse it and get all the deps
// // if a dep does not exist, do we care? for now forget about it
// // how would it work basically?
// // I could first create a graph, and give it to each language specific parser
// // this parser would then add edges in the graph for now, simple

// // there are two operations, who are all my dependencies
// // and my dependents

// // but i do need to find all the packages and create a basic graph, is this okay?
// // can i start from a given packages for graph building?
// // downstream: requires getting everything
// // upstream: simply get the dependencies from the interface, and call it recursively for them

// // so anyways, it would be good to have a way of incrementally adding a graph
// // given a graph, for any package -> add the package to the graph
// // add the dependencies to the graph
// // add edges from us to dependencies
// // run the code for all dependencies
// // its best to pass, who all are done
// // cycle detection is also needed

// fn build_dependency_graph(pkgs: Vec<Package>) -> Graph<Package, ()> {
//     build_dependency_graph_inner(pkgs, get_dependencies)
// }

// fn build_dependency_graph_inner<F>(pkgs: Vec<Package>, get_deps: F) -> Graph<Package, ()>
// where
//     F: Fn(&Package) -> Vec<Package>,
// {
//     let mut graph = Graph::new();
//     let mut done = HashMap::new();
//     for pkg in pkgs {
//         build_dependency_graph_rec(&mut graph, pkg, &mut done, &get_deps);
//     }
//     graph
// }

// // given a graph and a node
// // if the node exists in the graph and is done, return
// // or simply build graph for dependencies
// // add edge
// fn build_dependency_graph_rec<F>(
//     g: &mut Graph<Package, ()>,
//     pkg: Package,
//     done: &mut HashMap<Package, NodeIndex>,
//     get_deps: &F,
// ) -> NodeIndex
// where
//     F: Fn(&Package) -> Vec<Package>,
// {
//     println!("doing: {:?}", pkg);
//     if done.contains_key(&pkg) {
//         return done[&pkg];
//     }
//     let us = g.add_node(pkg.clone());
//     done.insert(pkg.clone(), us.clone());
//     for dep in get_deps(&pkg) {
//         let them = build_dependency_graph_rec(g, dep.clone(), done, get_deps);
//         println!("adding edge: {:?} -> {:?}", pkg, dep);
//         g.add_edge(us, them, ());
//     }
//     us
// }

// fn get_dependencies(pkg: &Package) -> Vec<Package> {
//     vec![]
// }

// #[derive(Debug, Clone, Hash, Eq, PartialEq)]
// pub struct Package {
//     pub name: String,
// }


// mod test {
//     use petgraph::dot::Dot;

//     use super::*;

//     #[test]
//     fn test_build_graph() {
//         let get_deps = |pkg: &Package| {
//             let deps: HashMap<String, Vec<Package>> = HashMap::from([
//                 (
//                     "cathode".to_string(),
//                     vec!["qxr", "qxr_reports", "qxr_blaze"]
//                         .iter()
//                         .map(|s| Package {
//                             name: s.to_string(),
//                         })
//                         .collect(),
//                 ),
//                 (
//                     "qxr_blaze".to_string(),
//                     vec!["qxr", "qxr_reports"]
//                         .iter()
//                         .map(|s| Package {
//                             name: s.to_string(),
//                         })
//                         .collect(),
//                 ),
//                 ("qxr_reports".to_string(), vec![]),
//             ]);
//             deps.get(&pkg.name).cloned().unwrap_or_else(|| vec![])
//         };

//         let graph = build_dependency_graph_inner(
//             vec![Package {
//                 name: "cathode".to_string(),
//             }],
//             get_deps,
//         );

//         graph.node_weight(Package{name: "cathode".to_string()});
//         assert_eq!(graph.node_count(), 4);
//         assert_eq!(graph.edge_count(), 5);
//     }
// }
